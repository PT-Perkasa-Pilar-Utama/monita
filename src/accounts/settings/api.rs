//! A user's own settings as JSON. Contract: docs/api-specs/02-authentication.md section 2.5.

use axum::{
    Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::post,
};
use serde::Deserialize;

use super::SettingsError;
use crate::{
    accounts::{AccountUser, session_token},
    app::AppState,
    platform::{
        now,
        ui::{Body, UiError, UiResult},
    },
};

pub fn router() -> Router<AppState> {
    Router::new().route("/api/ui/me/password", post(password))
}

impl From<SettingsError> for UiError {
    fn from(e: SettingsError) -> Self {
        match e {
            SettingsError::NoCurrent | SettingsError::WrongPassword => {
                UiError::field("current", e.to_string())
            }
            SettingsError::TooShort => UiError::field("new", e.to_string()),
            SettingsError::TooMany(_) => UiError::TooMany(e.to_string()),
            SettingsError::Other(e) => UiError::Internal(e),
        }
    }
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct PasswordForm {
    current: String,
    new: String,
}

/// 204. This session stays signed in; every other one ends.
async fn password(
    State(state): State<AppState>,
    AccountUser(user): AccountUser,
    headers: HeaderMap,
    Body(f): Body<PasswordForm>,
) -> UiResult {
    let raw = session_token(&headers).unwrap_or_default();
    super::change_password(
        &state.db,
        &state.limiter,
        user.id,
        &raw,
        &f.current,
        &f.new,
        now(),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
