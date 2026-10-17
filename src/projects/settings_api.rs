//! A project's settings as JSON: owner, description and icon, API keys, allowlist.
//! Alerts are in `notify::api`. Contract: docs/api-specs/03-projects.md.

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, put},
};
use serde::{Deserialize, Serialize};

use super::{ProfileInput, ProjectError, api::form_error};
use crate::{
    accounts::{AdminUser, CurrentUser},
    app::AppState,
    platform::ui::{UiError, UiResult, created, data},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/ui/projects/{slug}/profile", put(profile))
        .route("/api/ui/projects/{slug}/owners", get(owners))
        .route("/api/ui/projects/{slug}/owner", put(reassign))
        .route("/api/ui/projects/{slug}/keys", get(keys).post(create_key))
        .route("/api/ui/projects/{slug}/keys/{id}/revoke", post(revoke_key))
        .route("/api/ui/projects/{slug}/allowlist", put(allowlist))
}

fn no_content() -> UiResult {
    Ok(StatusCode::NO_CONTENT.into_response())
}

/// The owner, or a super admin, saves the description and sets, keeps or removes the icon.
async fn profile(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
    Json(input): Json<ProfileInput>,
) -> UiResult {
    let p = super::find_for(&state.db, &user, &slug).await?;
    super::update_profile(&state.db, p.id, &input)
        .await
        .map_err(form_error)?;
    no_content()
}

/// Who a super admin may make the owner: themself, then every active tech lead.
async fn owners(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
    Path(slug): Path<String>,
) -> UiResult {
    super::find_for(&state.db, &admin, &slug).await?;
    Ok(data(super::api::owners(&state, &admin).await?))
}

#[derive(Deserialize)]
struct OwnerInput {
    owner_email: String,
}

/// AC-05.01: a super admin gives the project to an eligible owner, named by email.
async fn reassign(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
    Path(slug): Path<String>,
    Json(f): Json<OwnerInput>,
) -> UiResult {
    let p = super::find_for(&state.db, &admin, &slug).await?;
    let Some(owner_id) = super::find_owner(&state.db, &admin, &f.owner_email).await? else {
        return Err(UiError::field(
            "owner_email",
            ProjectError::UnknownOwner.to_string(),
        ));
    };
    super::reassign(&state.db, p.id, owner_id, &admin).await?;
    no_content()
}

async fn keys(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
) -> UiResult {
    let p = super::find_for(&state.db, &user, &slug).await?;
    Ok(data(super::list_keys(&state.db, p.id).await?))
}

#[derive(Deserialize)]
struct KeyInput {
    name: String,
}

/// The full key travels once, in this answer; only its hash is stored.
#[derive(Serialize)]
struct NewKey {
    key: String,
}

async fn create_key(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
    Json(f): Json<KeyInput>,
) -> UiResult {
    let p = super::find_for(&state.db, &user, &slug).await?;
    let key = super::create_key(&state.db, p.id, &f.name)
        .await
        .map_err(form_error)?;
    Ok(created(NewKey { key }))
}

async fn revoke_key(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, id)): Path<(String, i64)>,
) -> UiResult {
    let p = super::find_for(&state.db, &user, &slug).await?;
    super::revoke_key(&state.db, p.id, id).await?;
    no_content()
}

#[derive(Deserialize)]
struct AllowlistInput {
    cidrs: String,
}

/// US-08: an empty value turns the allowlist off.
async fn allowlist(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
    Json(f): Json<AllowlistInput>,
) -> UiResult {
    let p = super::find_for(&state.db, &user, &slug).await?;
    super::set_allowlist(&state.db, p.id, &f.cidrs)
        .await
        .map_err(form_error)?;
    no_content()
}
