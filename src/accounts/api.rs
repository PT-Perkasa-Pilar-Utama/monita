//! Sign-in, sign-out, the signed-in user and invites, as JSON for the frontend.
//! Contract: docs/api-specs/02-authentication.md.

use axum::{
    Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};

use super::{AccountError, CurrentUser};
use crate::{
    app::AppState,
    platform::ui::{Body, UiError, UiResult, data},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/ui/login", post(login))
        .route("/api/ui/logout", post(logout))
        .route("/api/ui/me", get(me))
        .route("/api/ui/invites/{token}", get(invite).post(accept))
}

fn secure(state: &AppState) -> bool {
    state.config.base_url.starts_with("https://")
}

/// 204 with the session cookie.
fn signed_in(state: &AppState, raw: &str) -> Response {
    (
        StatusCode::NO_CONTENT,
        [(
            header::SET_COOKIE,
            super::session_cookie(raw, secure(state)),
        )],
    )
        .into_response()
}

#[derive(Serialize)]
pub struct Me {
    pub email: String,
    pub role: &'static str,
    /// Changes anything: a super admin.
    pub is_admin: bool,
    /// Sees every project and the admin pages: a super admin or a supervisor.
    pub sees_all: bool,
    /// Reads only: a supervisor.
    pub read_only: bool,
    /// `MONITA_BASE_URL`: the address people outside use, for links and examples a page shows.
    /// It can differ from the address the browser used, as behind a proxy.
    pub base_url: String,
    /// Minutes east of UTC that pages show times in.
    pub utc_offset_min: i64,
}

async fn me(State(state): State<AppState>, CurrentUser(user): CurrentUser) -> UiResult {
    let utc_offset_min = crate::platform::settings::utc_offset(&state.db.read().await?).await?;
    Ok(data(Me {
        utc_offset_min,
        is_admin: user.is_admin(),
        sees_all: user.sees_all(),
        read_only: user.is_read_only(),
        base_url: state.config.base_url.clone(),
        role: user.role.as_str(),
        email: user.email,
    }))
}

#[derive(Deserialize)]
struct LoginInput {
    email: String,
    password: String,
}

async fn login(State(state): State<AppState>, Body(f): Body<LoginInput>) -> UiResult {
    match super::login(&state.db, &f.email, &f.password).await {
        Ok(raw) => Ok(signed_in(&state, &raw)),
        Err(AccountError::Other(e)) => Err(e.into()),
        Err(e) => Err(UiError::invalid(e.to_string())),
    }
}

async fn logout(State(state): State<AppState>, headers: HeaderMap) -> UiResult {
    if let Some(raw) = super::session_token(&headers) {
        super::logout(&state.db, &raw).await?;
    }
    Ok((
        StatusCode::NO_CONTENT,
        [(header::SET_COOKIE, super::clear_cookie())],
    )
        .into_response())
}

/// 200 while the link can set a password, 410 after.
async fn invite(State(state): State<AppState>, Path(token): Path<String>) -> UiResult {
    if super::invite_is_usable(&state.db, &token).await? {
        Ok(data(serde_json::json!({ "usable": true })))
    } else {
        Err(UiError::Gone(AccountError::InviteInvalid.to_string()))
    }
}

#[derive(Deserialize)]
struct InviteInput {
    password: String,
    password_confirm: String,
}

async fn accept(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Body(f): Body<InviteInput>,
) -> UiResult {
    // api-specs 02: a dead link is 410 before any other check.
    if !super::invite_is_usable(&state.db, &token).await? {
        return Err(UiError::Gone(AccountError::InviteInvalid.to_string()));
    }
    if f.password != f.password_confirm {
        return Err(UiError::field(
            "password_confirm",
            "The two passwords do not match.",
        ));
    }
    match super::accept_invite(&state.db, &token, &f.password).await {
        Ok(raw) => Ok(signed_in(&state, &raw)),
        Err(AccountError::InviteInvalid) => {
            Err(UiError::Gone(AccountError::InviteInvalid.to_string()))
        }
        Err(AccountError::Other(e)) => Err(e.into()),
        Err(e) => Err(UiError::field("password", e.to_string())),
    }
}
