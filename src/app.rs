use std::sync::{Arc, atomic::AtomicI64};

use axum::{
    Router, middleware,
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
}

/// Opens the database, migrates it and starts the background loops.
pub async fn build(config: Config) -> anyhow::Result<(Router, AppState)> {
    build_with(config, Mailer::Log).await
}

/// `build` with a chosen mail adapter. Tests pass `Mailer::memory()`.
pub async fn build_with(config: Config, mailer: Mailer) -> anyhow::Result<(Router, AppState)> {
    if let Some(dir) = config.db_path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let db = Db::open(&config.db_path).await?;
    migrate::run(&db).await?;
    let state = AppState {
        db,
        config: Arc::new(config),
        last_tick: Arc::new(AtomicI64::new(0)),
        mailer,
        http: crate::checks::Clients::new()?,
        limiter: Arc::default(),
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
        .merge(crate::platform::limits::router());

    // SEC-17: the route does not exist in prod. Registration is the control, not authorization.
    if state.config.env != Env::Prod {
        r = r.route("/admin/reset-state", post(reset_state::handler));
    }

    r.layer(middleware::from_fn_with_state(
        state.clone(),
        headers::origin_check,
    ))
    .layer(middleware::from_fn(headers::security_headers))
    .with_state(state)
}

pub async fn serve(config: Config) -> anyhow::Result<()> {
    let bind = config.bind;
    let (router, _state) = build(config).await?;
    let listener = TcpListener::bind(bind).await?;
    tracing::info!(%bind, "monita listening");
    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;
    Ok(())
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
