use std::sync::{Arc, atomic::AtomicI64};

use axum::{
    Router,
    extract::DefaultBodyLimit,
    middleware,
    routing::{get, post},
};
use tokio::net::TcpListener;

use crate::notify::Mailer;
use crate::platform::{
    assets,
    config::{Config, Env},
    db::Db,
    headers, health, migrate, reset_state,
};

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub config: Arc<Config>,
    /// Unix seconds of the scheduler's last tick. `/healthz` reads it.
    pub last_tick: Arc<AtomicI64>,
    pub mailer: Mailer,
    pub http: crate::checks::Clients,
    pub limiter: Arc<crate::platform::rate_limit::Limiter>,
    /// Background work holds a read guard; reset-state and shutdown take the write guard, so
    /// neither races a check, a delivery or a daily job that is mid-write.
    pub gate: Arc<tokio::sync::RwLock<()>>,
}

/// Opens the database, migrates it and starts the background loops, with a chosen mail
/// adapter. Tests pass `Mailer::memory()`.
pub async fn build_with(config: Config, mailer: Mailer) -> anyhow::Result<(Router, AppState)> {
    let (router, state, _scheduler) = start(config, mailer).await?;
    Ok((router, state))
}

async fn start(
    config: Config,
    mailer: Mailer,
) -> anyhow::Result<(Router, AppState, tokio::task::JoinHandle<()>)> {
    let db = Db::open(&config.db_path).await?;
    migrate::run(&db).await?;
    let state = AppState {
        db,
        config: Arc::new(config),
        last_tick: Arc::new(AtomicI64::new(0)),
        mailer,
        http: crate::checks::Clients::new()?,
        limiter: Arc::default(),
        gate: Arc::default(),
    };
    let scheduler = crate::checks::scheduler::spawn(state.clone());
    Ok((router(state.clone()), state, scheduler))
}

/// SEC-19: forms get 413 past this. `/fail` has its own bound in `ping_routes`.
pub const MAX_BODY: usize = 128 * 1024;

pub fn router(state: AppState) -> Router {
    let mut r = Router::new()
        .route("/healthz", get(health::healthz))
        .route("/static/{*path}", get(assets::serve))
        .merge(crate::accounts::routes::router())
        .merge(crate::projects::routes::router())
        .merge(crate::tickets::routes::router())
        .merge(crate::checks::routes::router())
        .merge(crate::checks::heartbeat_routes::router())
        .merge(crate::checks::ping_routes::router())
        .merge(crate::incidents::routes::router())
        .merge(crate::notify::routes::router())
        .merge(crate::status::routes::router())
        .merge(crate::platform::limits::router());

    // SEC-17: the route does not exist in prod. Registration is the control, not authorization.
    if state.config.env != Env::Prod {
        r = r.route("/admin/reset-state", post(reset_state::handler));
    }

    // 128 KiB: the largest valid form is a ticket of 10 000 four-byte characters, percent-encoded.
    r.layer(DefaultBodyLimit::max(MAX_BODY))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            crate::platform::rate_limit::per_ip,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            headers::origin_check,
        ))
        .layer(middleware::from_fn(headers::security_headers))
        .with_state(state)
}

pub async fn serve(config: Config) -> anyhow::Result<()> {
    let bind = config.bind;
    // Bind first: a second instance on a taken port must fail before its loops touch the DB.
    let listener = TcpListener::bind(bind).await?;
    let mailer = Mailer::from_config(&config)?;
    let (router, state, scheduler) = start(config, mailer).await?;
    // HTTP would keep serving without the loop, so systemd would never restart a dead scheduler.
    // A panic or cancel drains running work, then ends the process; `Restart=on-failure`
    // starts it again.
    // A panic in a delivery or daily-job task needs none of this: its drop guard frees the
    // running flag and the next tick starts it again.
    let gate = state.gate.clone();
    tokio::spawn(async move {
        if scheduler_died(scheduler, &gate).await {
            std::process::exit(1);
        }
    });
    tracing::info!(%bind, "monita listening");
    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;
    // Spec 13.4: the write guard stops the scheduler, whose next tick waits behind it, so it must
    // stay held until the process exits.
    let held = drain(&state.gate, std::time::Duration::from_secs(10)).await;
    if held.is_none() {
        tracing::warn!("background work still running after 10 s; exiting anyway");
    }
    Ok(())
}

/// Waits for the scheduler loop. True when it ended abnormally (a panic or a cancel); running
/// work is drained first, so the caller can exit at once.
async fn scheduler_died(
    scheduler: tokio::task::JoinHandle<()>,
    gate: &Arc<tokio::sync::RwLock<()>>,
) -> bool {
    match scheduler.await {
        Err(e) => {
            tracing::error!(error = %e, "the scheduler stopped; exiting so the service restarts");
            let _held = drain(gate, std::time::Duration::from_secs(10)).await;
            true
        }
        _ => false,
    }
}

/// Waits up to `limit` for every probe, delivery pass and daily job to finish, and returns the
/// write guard that keeps new ones from starting.
pub async fn drain(
    gate: &Arc<tokio::sync::RwLock<()>>,
    limit: std::time::Duration,
) -> Option<tokio::sync::OwnedRwLockWriteGuard<()>> {
    tokio::time::timeout(limit, gate.clone().write_owned())
        .await
        .ok()
}

async fn shutdown_signal() {
    // A handler that cannot be installed must wait forever, not end the server at start.
    let ctrl_c = async {
        if let Err(e) = tokio::signal::ctrl_c().await {
            tracing::error!(error = %e, "no Ctrl-C handler");
            std::future::pending::<()>().await;
        }
    };
    #[cfg(unix)]
    let term = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut s) => {
                s.recv().await;
            }
            Err(e) => {
                tracing::error!(error = %e, "no SIGTERM handler");
                std::future::pending::<()>().await;
            }
        }
    };
    #[cfg(not(unix))]
    let term = std::future::pending::<()>();
    tokio::select! { _ = ctrl_c => {}, _ = term => {} }
    tracing::info!("shutting down");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn a_scheduler_panic_is_reported_after_running_work_drains() {
        let gate: Arc<tokio::sync::RwLock<()>> = Arc::default();
        let running = gate.clone().read_owned().await;
        let done = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = done.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            flag.store(true, std::sync::atomic::Ordering::SeqCst);
            drop(running);
        });
        let panicked = tokio::spawn(async { panic!("scheduler bug") });
        assert!(scheduler_died(panicked, &gate).await);
        assert!(
            done.load(std::sync::atomic::Ordering::SeqCst),
            "the running probe finished first"
        );
        let finished = tokio::spawn(async {});
        assert!(!scheduler_died(finished, &gate).await);
        let cancelled = tokio::spawn(std::future::pending::<()>());
        cancelled.abort();
        assert!(
            scheduler_died(cancelled, &gate).await,
            "a cancel ends the loop too"
        );
    }

    #[tokio::test]
    async fn drain_waits_for_running_work_but_not_forever() {
        let gate: Arc<tokio::sync::RwLock<()>> = Arc::default();
        let running = gate.clone().read_owned().await;
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            drop(running);
        });
        let held = drain(&gate, Duration::from_secs(2)).await;
        assert!(held.is_some());
        assert!(
            gate.try_read().is_err(),
            "nothing new starts while the guard is held"
        );

        let gate: Arc<tokio::sync::RwLock<()>> = Arc::default();
        let _stuck = gate.clone().read_owned().await;
        assert!(drain(&gate, Duration::from_millis(50)).await.is_none());
    }
}
