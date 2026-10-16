//! `POST /admin/reset-state`. Spec: docs/api-specs/09-system.md section 9.2.
//! Mounted only outside prod (see `app::router`).

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Deserialize;

use super::{config::Env, migrate};
use crate::app::AppState;

#[derive(Deserialize)]
pub struct ResetRequest {
    seed: String,
}

pub async fn handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<ResetRequest>,
) -> Response {
    // ponytail: dev accepts the reset token too, until sessions exist (US-03); then dev needs a super admin session.
    let token_ok = match (&state.config.reset_token, headers.get("x-reset-token")) {
        (Some(expected), Some(got)) => got.as_bytes() == expected.as_bytes(),
        _ => false,
    };
    if state.config.env == Env::Prod || !token_ok {
        return error(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "Missing or wrong reset token.",
        );
    }
    let seed = match req.seed.as_str() {
        "dev" => include_str!("../../seeds/dev.sql"),
        "qa" => include_str!("../../seeds/qa.sql"),
        _ => {
            return error(
                StatusCode::BAD_REQUEST,
                "invalid_request",
                "seed must be dev or qa.",
            );
        }
    };
    // ponytail: the scheduler is not paused yet; it only ticks until US-09 adds checks. Pause it then.
    let result = async {
        state.db.drop_all_tables().await?;
        migrate::run(&state.db).await?;
        state
            .db
            .write(async |c| {
                c.execute_batch(seed).await?;
                Ok(())
            })
            .await
    }
    .await;
    match result {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => {
            tracing::error!(error = %e, "reset-state failed");
            error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal",
                "Reset failed. See the server log.",
            )
        }
    }
}

fn error(status: StatusCode, code: &str, message: &str) -> Response {
    (
        status,
        Json(serde_json::json!({ "error": { "code": code, "message": message } })),
    )
        .into_response()
}
