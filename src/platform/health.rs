//! `GET /healthz`. Spec: docs/api-specs/09-system.md section 9.1.

use std::sync::atomic::Ordering;

use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::Serialize;

use crate::app::AppState;

/// The scheduler ticks every second. Longer than this means the loop is stuck.
const MAX_SCHEDULER_LAG_S: i64 = 30;

#[derive(Serialize)]
struct Health {
    status: &'static str,
    db: &'static str,
    scheduler_lag_s: i64,
    outbox_pending: Option<i64>,
    version: &'static str,
}

/// One query proves the database answers and reads the outbox backlog: rows not yet sent or
/// failed, retries included, so SMTP being down shows as a growing number.
async fn probe(state: &AppState) -> anyhow::Result<i64> {
    crate::notify::pending_count(&state.db.read().await?).await
}

pub async fn healthz(State(state): State<AppState>) -> impl IntoResponse {
    let (db_ok, outbox_pending) = match probe(&state).await {
        Ok(pending) => (true, Some(pending)),
        Err(e) => {
            tracing::error!(error = %e, "health check: database unavailable");
            (false, None)
        }
    };
    let lag = super::now() - state.last_tick.load(Ordering::Relaxed);
    let ok = db_ok && lag <= MAX_SCHEDULER_LAG_S;
    let body = Health {
        status: if ok { "ok" } else { "fail" },
        db: if db_ok { "ok" } else { "fail" },
        scheduler_lag_s: lag,
        outbox_pending,
        version: env!("CARGO_PKG_VERSION"),
    };
    let code = if ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (code, Json(body))
}
