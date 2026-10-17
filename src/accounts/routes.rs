//! Sign-in, invites and TL management. Contract: docs/api-specs/02-authentication.md.

use askama::Template;
use axum::{
    Form, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use serde::Deserialize;

use super::{AccountError, AdminUser, UserRow};
use crate::{
    app::AppState,
    platform::{
        page::{PageError, Shell},
        render::render,
        time,
    },
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/admin/users", get(users_page).post(users_create))
        .route("/admin/users/{id}/invite", post(users_reinvite))
        .route("/admin/users/{id}/disable", post(users_disable))
}

#[derive(Template)]
#[template(path = "accounts/users.html")]
struct UsersPage {
    shell: Shell,
    users: Vec<UserRow>,
    email: String,
    error: String,
    invite: Option<(String, String)>,
}

impl UsersPage {
    fn joined(&self, u: &UserRow) -> String {
        time::utc(u.created_at)
    }
}

async fn users_view(
    state: &AppState,
    admin: &super::User,
    email: String,
    error: String,
    invite: Option<(String, String)>,
) -> Result<UsersPage, PageError> {
    Ok(UsersPage {
        shell: Shell::plain(&admin.email, true, "users"),
        users: super::list_users(&state.db).await?,
        email,
        error,
        invite,
    })
}

async fn users_page(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
) -> Result<Response, PageError> {
    Ok(render(
        &users_view(&state, &admin, String::new(), String::new(), None).await?,
    ))
}

#[derive(Deserialize)]
struct NewUserForm {
    email: String,
}

async fn users_create(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
    Form(f): Form<NewUserForm>,
) -> Result<Response, PageError> {
    match super::invite_tl(&state.db, &state.config.base_url, &f.email).await {
        Ok(link) => {
            let page = users_view(
                &state,
                &admin,
                String::new(),
                String::new(),
                Some((f.email.trim().to_lowercase(), link)),
            )
            .await?;
            Ok(render(&page))
        }
        Err(AccountError::Other(e)) => Err(e.into()),
        Err(e) => {
            let page = users_view(&state, &admin, f.email, e.to_string(), None).await?;
            Ok((StatusCode::UNPROCESSABLE_ENTITY, render(&page)).into_response())
        }
    }
}

async fn users_reinvite(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
    Path(id): Path<i64>,
) -> Result<Response, PageError> {
    let invite = super::reinvite(&state.db, &state.config.base_url, id).await?;
    Ok(render(
        &users_view(&state, &admin, String::new(), String::new(), Some(invite)).await?,
    ))
}

async fn users_disable(
    State(state): State<AppState>,
    _: AdminUser,
    Path(id): Path<i64>,
) -> Result<Response, PageError> {
    super::disable(&state.db, id).await?;
    Ok(Redirect::to("/admin/users").into_response())
}
