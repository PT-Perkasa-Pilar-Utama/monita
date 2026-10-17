//! The Users page as JSON: tech leads, invites and disable. Super admin only.
//! Contract: docs/api-specs/02-authentication.md.

use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};

use super::{AccountError, AdminUser, UserRow};
use crate::{
    app::AppState,
    platform::ui::{Body, UiError, UiResult, created, data},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/ui/admin/users", get(list).post(invite))
        .route("/api/ui/admin/users/{id}/invite", post(reinvite))
        .route("/api/ui/admin/users/{id}/disable", post(disable))
}

/// A user as the page shows them. Never the password hash.
#[derive(Serialize)]
struct UserView {
    id: i64,
    email: String,
    role: String,
    /// `active`, `invited` (no password yet) or `disabled`.
    status: &'static str,
    created_at: i64,
}

impl From<UserRow> for UserView {
    fn from(u: UserRow) -> Self {
        let status = match (u.disabled_at, u.password_hash) {
            (Some(_), _) => "disabled",
            (None, None) => "invited",
            (None, Some(_)) => "active",
        };
        UserView {
            id: u.id,
            email: u.email,
            role: u.role,
            status,
            created_at: u.created_at,
        }
    }
}

async fn list(State(state): State<AppState>, _: AdminUser) -> UiResult {
    let users: Vec<UserView> = super::list_users(&state.db)
        .await?
        .into_iter()
        .map(UserView::from)
        .collect();
    Ok(data(users))
}

/// The invite link, shown once to the admin who made it; only its hash is stored.
#[derive(Serialize)]
struct Invited {
    email: String,
    link: String,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct InviteForm {
    email: String,
}

/// US-02. 201 with the link; a bad or taken email is 422 on `email`.
async fn invite(
    State(state): State<AppState>,
    _: AdminUser,
    Body(f): Body<InviteForm>,
) -> UiResult {
    match super::invite_tl(&state.db, &state.config.base_url, &f.email).await {
        Ok(link) => Ok(created(Invited {
            email: f.email.trim().to_lowercase(),
            link,
        })),
        Err(AccountError::Other(e)) => Err(UiError::Internal(e)),
        Err(e) => Err(UiError::field("email", e.to_string())),
    }
}

/// A new link for an active tech lead; the old one stops working.
async fn reinvite(State(state): State<AppState>, _: AdminUser, Path(id): Path<i64>) -> UiResult {
    let (email, link) = super::reinvite(&state.db, &state.config.base_url, id).await?;
    Ok(data(Invited { email, link }))
}

/// US-04. Signs them out at once. 204.
async fn disable(State(state): State<AppState>, _: AdminUser, Path(id): Path<i64>) -> UiResult {
    super::disable(&state.db, id).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
