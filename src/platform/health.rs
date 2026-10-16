//! `GET /healthz`. Spec: docs/api-specs/09-system.md section 9.1.

use std::sync::atomic::Ordering;

use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::Serialize;

use super::db::query_i64;
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

/// `SELECT 1` proves the database answers; the count is the outbox rows due and not yet sent.
async fn probe(state: &AppState, now: i64) -> anyhow::Result<i64> {
    let conn = state.db.read().await?;
    query_i64(&conn, "SELECT 1", ()).await?;
    Ok(query_i64(
        &conn,
        "SELECT count(*) FROM outbox WHERE sent_at IS NULL AND failed_at IS NULL AND next_attempt_at <= ?1",
        [now],
    )
    .await?
    .unwrap_or(0))
}

pub async fn healthz(State(state): State<AppState>) -> impl IntoResponse {
    let (db_ok, outbox_pending) = match probe(&state, super::now()).await {
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
