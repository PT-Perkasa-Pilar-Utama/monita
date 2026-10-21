//! Two-factor as JSON: the code step of sign-in, and a user's own two-factor settings.
//! Contract: docs/api-specs/02-authentication.md section 2.4.

use axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use serde::Deserialize;

use super::TwoFactorError;
use crate::{
    accounts::AccountUser,
    app::AppState,
    platform::{
        now,
        ui::{Body, UiError, UiResult, data},
    },
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/ui/login/code", post(login_code))
        .route("/api/ui/me/two-factor", get(status).post(start))
        .route("/api/ui/me/two-factor/confirm", post(confirm))
        .route("/api/ui/me/two-factor/off", post(off))
}

impl From<TwoFactorError> for UiError {
    fn from(e: TwoFactorError) -> Self {
        match e {
            TwoFactorError::WrongCode => UiError::field("code", e.to_string()),
            TwoFactorError::ChallengeGone => UiError::Gone(e.to_string()),
            TwoFactorError::TooMany(_) => UiError::TooMany(e.to_string()),
            TwoFactorError::Other(e) => UiError::Internal(e),
            TwoFactorError::AlreadyOn | TwoFactorError::NotOn | TwoFactorError::NotPending => {
                UiError::invalid(e.to_string())
            }
        }
    }
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct CodeStep {
    challenge: String,
    code: String,
}

/// 204 with the session cookie; 422 on `code` for a wrong code; 410 once the challenge is gone.
async fn login_code(State(state): State<AppState>, Body(f): Body<CodeStep>) -> UiResult {
    let raw = crate::accounts::login_code(&state.db, &state.limiter, &f.challenge, &f.code).await?;
    Ok(crate::accounts::api::signed_in(&state, &raw))
}

async fn status(State(state): State<AppState>, AccountUser(user): AccountUser) -> UiResult {
    Ok(data(super::status(&state.db, user.id).await?))
}

/// The secret, its `otpauth://` link and the link as a QR code. Nothing changes until `confirm`.
async fn start(State(state): State<AppState>, AccountUser(user): AccountUser) -> UiResult {
    Ok(data(
        super::start(&state.db, user.id, &user.email, now()).await?,
    ))
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Code {
    code: String,
}

/// Turns two-factor on. The recovery codes are in this answer only.
async fn confirm(
    State(state): State<AppState>,
    AccountUser(user): AccountUser,
    Body(f): Body<Code>,
) -> UiResult {
    let codes = super::confirm(&state.db, user.id, &f.code, now()).await?;
    Ok(data(serde_json::json!({ "recovery_codes": codes })))
}

async fn off(
    State(state): State<AppState>,
    AccountUser(user): AccountUser,
    Body(f): Body<Code>,
) -> UiResult {
    super::turn_off(&state.db, &state.limiter, user.id, &f.code, now()).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
