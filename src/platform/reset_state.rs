//! `POST /admin/reset-state`. Spec: docs/api-specs/09-system.md section 9.2.
//! Mounted only outside prod (see `app::router`).

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Deserialize;

use super::{api_error, config::Env, migrate};
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
    let token_ok = match (
        &state.config.reset_token,
        headers.get("x-reset-token").and_then(|v| v.to_str().ok()),
    ) {
        (Some(expected), Some(got)) => super::token::hash(got) == super::token::hash(expected),
        _ => false,
    };
    if state.config.env == Env::Prod || !token_ok {
        return api_error(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "Missing or wrong reset token.",
        );
    }
    let seed = match req.seed.as_str() {
        "dev" => include_str!("../../seeds/dev.sql"),
        "qa" => include_str!("../../seeds/qa.sql"),
        "demo" => include_str!("../../seeds/demo.sql"),
        _ => {
            return api_error(
                StatusCode::BAD_REQUEST,
                "invalid_request",
                "seed must be dev, qa or demo.",
            );
        }
    };
    // Waits for running checks and blocks new ones until the reset is done.
    let _exclusive = state.gate.write().await;
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
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal",
                "Reset failed. See the server log.",
            )
        }
    }
}
