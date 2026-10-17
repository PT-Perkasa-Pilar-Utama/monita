//! Project pages. Contract: docs/api-specs/03-projects.md.

use super::{Project, ProjectError, ProjectInput};
use crate::{
    accounts::{CurrentUser, User, UserRow},
    app::AppState,
    platform::{
        page::{PageError, Shell},
        render::render,
    },
};
use askama::Template;
use axum::{
    Form, Router,
    extract::{Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(home))
        .route("/projects", post(create))
        .route("/projects/{slug}", get(super::overview::page))
        .route("/projects/{slug}/icon", get(icon))
        .merge(super::settings_routes::router())
}

#[derive(Template)]
#[template(path = "projects/home.html")]
struct Home {
    shell: Shell,
    projects: Vec<Project>,
    /// Who a super admin may name as owner; empty for a tech lead.
    owners: Vec<UserRow>,
    input: ProjectInput,
    error: String,
}

impl Home {
    fn initial(&self, p: &Project) -> char {
        p.name.chars().next().unwrap_or('?').to_ascii_uppercase()
    }
    /// The owner field starts with the admin's own email, the owner when it is left as is.
    fn owner_value(&self) -> &str {
        match self.input.owner_email.trim() {
            "" => &self.shell.email,
            v => v,
        }
    }
}

async fn home_view(
    state: &AppState,
    user: &User,
    input: ProjectInput,
    error: String,
) -> Result<Home, PageError> {
    let owners = if user.is_admin() {
        super::eligible_owners(&state.db, user).await?
    } else {
        Vec::new()
    };
    Ok(Home {
        shell: super::shell(&state.db, user, "projects", None).await?,
        projects: super::list_for(&state.db, user).await?,
        owners,
        input,
        error,
    })
}

async fn home(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> Result<Response, PageError> {
    Ok(render(
        &home_view(&state, &user, ProjectInput::default(), String::new()).await?,
    ))
}

async fn create(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Form(input): Form<ProjectInput>,
) -> Result<Response, PageError> {
    match super::create(&state.db, &user, &input).await {
        Ok(p) => Ok(Redirect::to(&format!("/projects/{}/monitors", p.slug)).into_response()),
        Err(ProjectError::Other(e)) => Err(e.into()),
        Err(e) => {
            let page = home_view(&state, &user, input, e.to_string()).await?;
            Ok((StatusCode::UNPROCESSABLE_ENTITY, render(&page)).into_response())
        }
    }
}

/// The icon's URL carries `?v={icon_at}`, so a new icon is a new URL and the old one never changes.
async fn icon(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
) -> Result<Response, PageError> {
    let project = super::find_for(&state.db, &user, &slug).await?;
    let (mime, data) = super::icon(&state.db, project.id)
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
