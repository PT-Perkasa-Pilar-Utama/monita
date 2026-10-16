//! Sign-in, invites and TL management. Contract: docs/api-specs/02-authentication.md.

use askama::Template;
use axum::{
    Form, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
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
        .route("/login", get(login_page).post(login_submit))
        .route("/logout", post(logout))
        .route("/invite/{token}", get(invite_page).post(invite_submit))
        .route("/admin/users", get(users_page).post(users_create))
        .route("/admin/users/{id}/invite", post(users_reinvite))
        .route("/admin/users/{id}/disable", post(users_disable))
}

fn secure(state: &AppState) -> bool {
    state.config.base_url.starts_with("https://")
}

fn signed_in(state: &AppState, raw: &str) -> Response {
    (
        StatusCode::SEE_OTHER,
        [
            (
                header::SET_COOKIE,
                super::session_cookie(raw, secure(state)),
            ),
            (header::LOCATION, "/".to_string()),
        ],
    )
        .into_response()
}

#[derive(Template)]
#[template(path = "accounts/login.html")]
struct LoginPage<'a> {
    email: &'a str,
    error: Option<String>,
}

async fn login_page() -> Response {
    render(&LoginPage {
        email: "",
        error: None,
    })
}

#[derive(Deserialize)]
struct LoginForm {
    email: String,
    password: String,
}

async fn login_submit(
    State(state): State<AppState>,
    Form(f): Form<LoginForm>,
) -> Result<Response, PageError> {
    match super::login(&state.db, &f.email, &f.password).await {
        Ok(raw) => Ok(signed_in(&state, &raw)),
        Err(AccountError::Other(e)) => Err(e.into()),
        Err(e) => Ok((
            StatusCode::UNPROCESSABLE_ENTITY,
            render(&LoginPage {
                email: &f.email,
                error: Some(e.to_string()),
            }),
        )
            .into_response()),
    }
}

async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Result<Response, PageError> {
    if let Some(raw) = super::session_token(&headers) {
        super::logout(&state.db, &raw).await?;
    }
    Ok((
        StatusCode::SEE_OTHER,
        [
            (header::SET_COOKIE, super::clear_cookie()),
            (header::LOCATION, "/login".to_string()),
        ],
    )
        .into_response())
}

#[derive(Template)]
#[template(path = "accounts/invite.html")]
struct InvitePage {
    usable: bool,
    error: Option<String>,
}

async fn invite_page(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Response, PageError> {
    let usable = super::invite_is_usable(&state.db, &token).await?;
    let status = if usable {
        StatusCode::OK
    } else {
        StatusCode::GONE
    };
    Ok((
        status,
        render(&InvitePage {
            usable,
            error: None,
        }),
    )
        .into_response())
}

#[derive(Deserialize)]
struct InviteForm {
    password: String,
    password_confirm: String,
}

async fn invite_submit(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Form(f): Form<InviteForm>,
) -> Result<Response, PageError> {
    let invalid = |status, msg: String| {
        (
            status,
            render(&InvitePage {
                usable: status != StatusCode::GONE,
                error: Some(msg),
            }),
        )
            .into_response()
    };
    // api-specs 02: a dead link is 410 before any other check.
    if !super::invite_is_usable(&state.db, &token).await? {
        return Ok(invalid(
            StatusCode::GONE,
            AccountError::InviteInvalid.to_string(),
        ));
    }
    if f.password != f.password_confirm {
        return Ok(invalid(
            StatusCode::UNPROCESSABLE_ENTITY,
            "The two passwords do not match.".into(),
        ));
    }
    match super::accept_invite(&state.db, &token, &f.password).await {
        Ok(raw) => Ok(signed_in(&state, &raw)),
        Err(AccountError::InviteInvalid) => Ok(invalid(
            StatusCode::GONE,
            AccountError::InviteInvalid.to_string(),
        )),
        Err(AccountError::Other(e)) => Err(e.into()),
        Err(e) => Ok(invalid(StatusCode::UNPROCESSABLE_ENTITY, e.to_string())),
    }
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
        shell: Shell {
            email: admin.email.clone(),
            is_admin: true,
            active: "users",
            project: None,
        },
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
