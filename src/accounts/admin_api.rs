//! The Users page as JSON: accounts, roles, invites, disable and enable. A super admin changes;
//! a supervisor reads (their writes stop in `CurrentUser`).
//! Contract: docs/api-specs/02-authentication.md.

use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, put},
};
use serde::{Deserialize, Serialize};

use super::{AccountError, AdminError, AdminUser, Role, UserRow};
use crate::{
    app::AppState,
    platform::ui::{Body, UiError, UiResult, created, data},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/ui/admin/users", get(list).post(invite))
        .route("/api/ui/admin/users/{id}/invite", post(reinvite))
        .route("/api/ui/admin/users/{id}/disable", post(disable))
        .route("/api/ui/admin/users/{id}/enable", post(enable))
        .route("/api/ui/admin/users/{id}/role", put(set_role))
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
    /// `tl` (the default), `supervisor` or `super_admin`.
    role: String,
}

fn role_of(raw: &str) -> Result<Role, UiError> {
    match raw {
        "" => Ok(Role::Tl),
        r => Role::parse(r)
            .ok_or_else(|| UiError::field("role", "Pick tech lead, supervisor or super admin.")),
    }
}

impl From<AdminError> for UiError {
    fn from(e: AdminError) -> Self {
        match e {
            AdminError::Yourself => UiError::invalid(e.to_string()),
            AdminError::NotFound => UiError::NotFound,
            AdminError::OwnsProjects => UiError::field("role", e.to_string()),
            AdminError::NotSuperAdmin => UiError::Forbidden,
            AdminError::Other(e) => UiError::Internal(e),
        }
    }
}

/// US-02. 201 with the link; a bad or taken email is 422 on `email`.
async fn invite(
    State(state): State<AppState>,
    _: AdminUser,
    Body(f): Body<InviteForm>,
) -> UiResult {
    let role = role_of(&f.role)?;
    match super::invite(&state.db, &state.config.base_url, &f.email, role).await {
        Ok(link) => Ok(created(Invited {
            email: f.email.trim().to_lowercase(),
            link,
        })),
        Err(AccountError::Other(e)) => Err(UiError::Internal(e)),
        Err(e) => Err(UiError::field("email", e.to_string())),
    }
}

/// A new link for another active user; it resets their sign-in and the old link stops working.
async fn reinvite(
    State(state): State<AppState>,
    AdminUser(me): AdminUser,
    Path(id): Path<i64>,
) -> UiResult {
    let (email, link) = super::reset_sign_in(&state.db, &state.config.base_url, me.id, id).await?;
    Ok(data(Invited { email, link }))
}

/// US-04. Signs them out at once. 204.
async fn disable(
    State(state): State<AppState>,
    AdminUser(me): AdminUser,
    Path(id): Path<i64>,
) -> UiResult {
    super::disable(&state.db, me.id, id).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

/// Undoes a disable. 204.
async fn enable(State(state): State<AppState>, _: AdminUser, Path(id): Path<i64>) -> UiResult {
    super::enable(&state.db, id).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct RoleForm {
    role: String,
}

/// Makes another active user a super admin, a tech lead or a supervisor. 204; 422 on `role`
/// when a project owner would become a supervisor.
async fn set_role(
    State(state): State<AppState>,
    AdminUser(me): AdminUser,
    Path(id): Path<i64>,
    Body(f): Body<RoleForm>,
) -> UiResult {
    let role = Role::parse(&f.role)
        .ok_or_else(|| UiError::field("role", "Pick tech lead, supervisor or super admin."))?;
    super::set_role(&state.db, me.id, id, role).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
