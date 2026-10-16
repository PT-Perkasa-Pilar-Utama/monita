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

/// Opens the database, migrates it and starts the background loops.
pub async fn build(config: Config) -> anyhow::Result<(Router, AppState)> {
    let mailer = Mailer::from_config(&config)?;
    build_with(config, mailer).await
}

/// `build` with a chosen mail adapter. Tests pass `Mailer::memory()`.
pub async fn build_with(config: Config, mailer: Mailer) -> anyhow::Result<(Router, AppState)> {
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
    crate::checks::scheduler::spawn(state.clone());
    Ok((router(state.clone()), state))
}

pub fn router(state: AppState) -> Router {
    let mut r = Router::new()
        .route("/healthz", get(health::healthz))
        .route("/static/{*path}", get(assets::serve))
        .merge(crate::accounts::routes::router())
        .merge(crate::projects::routes::router())
        .merge(crate::tickets::routes::router())
        .merge(crate::checks::routes::router())
        .merge(crate::checks::heartbeat_routes::router())
        .merge(crate::incidents::routes::router())
        .merge(crate::notify::routes::router())
        .merge(crate::status::routes::router())
        .merge(crate::platform::limits::router());

    // SEC-17: the route does not exist in prod. Registration is the control, not authorization.
    if state.config.env != Env::Prod {
        r = r.route("/admin/reset-state", post(reset_state::handler));
    }

    // 128 KiB: the largest valid form is a ticket of 10 000 four-byte characters, percent-encoded.
    r.layer(DefaultBodyLimit::max(128 * 1024))
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
    let (router, state) = build(config).await?;
    let listener = TcpListener::bind(bind).await?;
    tracing::info!(%bind, "monita listening");
    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;
    // Spec 13.4: the held write guard also stops the scheduler, whose next tick waits behind it.
    if drain(&state.gate, std::time::Duration::from_secs(10))
        .await
        .is_none()
    {
        tracing::warn!("background work still running after 10 s; exiting anyway");
    }
    Ok(())
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
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let term = async {
        if let Ok(mut s) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            s.recv().await;
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
    async fn drain_waits_for_running_work_but_not_forever() {
        let gate: Arc<tokio::sync::RwLock<()>> = Arc::default();
        let running = gate.clone().read_owned().await;
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            drop(running);
        });
        assert!(drain(&gate, Duration::from_secs(2)).await.is_some());

        let gate: Arc<tokio::sync::RwLock<()>> = Arc::default();
        let _stuck = gate.clone().read_owned().await;
        assert!(drain(&gate, Duration::from_millis(50)).await.is_none());
    }
}
