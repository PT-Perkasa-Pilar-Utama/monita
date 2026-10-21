//! A user's own settings as JSON. Contract: docs/api-specs/02-authentication.md section 2.5.

use axum::{
    Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use serde::Deserialize;

use super::SettingsError;
use crate::{
    accounts::{AccountUser, session_token},
    app::AppState,
    platform::{
        now,
        page::PageError,
        ui::{Body, UiError, UiResult},
    },
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/ui/me/password", post(password))
        .route("/api/ui/me/avatar", put(set_avatar))
        .route("/users/{id}/avatar", get(avatar))
}

impl From<SettingsError> for UiError {
    fn from(e: SettingsError) -> Self {
        match e {
            SettingsError::NoCurrent | SettingsError::WrongPassword => {
                UiError::field("current", e.to_string())
            }
            SettingsError::TooShort => UiError::field("new", e.to_string()),
            SettingsError::BadPicture(m) => UiError::field("avatar", m),
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

#[derive(Deserialize, Default)]
#[serde(default)]
struct AvatarForm {
    /// Base64 of a PNG or WebP, with or without a `data:` prefix; empty removes the picture.
    avatar: String,
}

/// 204. The page reads `avatar_at` from `/api/ui/me` for the new URL.
async fn set_avatar(
    State(state): State<AppState>,
    AccountUser(user): AccountUser,
    Body(f): Body<AvatarForm>,
) -> UiResult {
    super::set_avatar(&state.db, user.id, &f.avatar, now()).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

/// The user's own picture. Its URL carries `?v={avatar_at}`, so a new picture is a new URL.
/// Any signed-in user sees the team's pictures: the top bar, the user list, a project's owner.
async fn avatar(
    State(state): State<AppState>,
    AccountUser(_): AccountUser,
    Path(id): Path<i64>,
) -> Result<Response, PageError> {
    let (mime, data) = super::avatar(&state.db, id)
        .await?
        .ok_or(PageError::NotFound)?;
    let headers = [
        (header::CONTENT_TYPE, mime),
        (
            header::CACHE_CONTROL,
            "private, max-age=31536000, immutable".to_string(),
        ),
    ];
    Ok((headers, data).into_response())
}
