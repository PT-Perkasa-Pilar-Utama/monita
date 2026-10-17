//! Project pages. Contract: docs/api-specs/03-projects.md.

use askama::Template;
use axum::{
    Form, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use serde::Deserialize;

use super::{Project, ProjectError};
use crate::{
    accounts::{CurrentUser, User},
    app::AppState,
    platform::{
        page::{PageError, Shell},
        render::render,
    },
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(home))
        .route("/projects", post(create))
        .route("/projects/{slug}", get(open))
        .merge(super::settings_routes::router())
}

#[derive(Template)]
#[template(path = "projects/home.html")]
struct Home {
    shell: Shell,
    projects: Vec<Project>,
    name: String,
    slug: String,
    error: String,
}

impl Home {
    fn initial(&self, p: &Project) -> char {
        p.name.chars().next().unwrap_or('?').to_ascii_uppercase()
    }
}

async fn home_view(
    state: &AppState,
    user: &User,
    name: String,
    slug: String,
    error: String,
) -> Result<Home, PageError> {
    Ok(Home {
        shell: super::shell(&state.db, user, "projects", None).await?,
        projects: super::list_for(&state.db, user).await?,
        name,
        slug,
        error,
    })
}

async fn home(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> Result<Response, PageError> {
    Ok(render(
        &home_view(&state, &user, String::new(), String::new(), String::new()).await?,
    ))
}

#[derive(Deserialize)]
struct NewProject {
    name: String,
    slug: String,
}

async fn create(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Form(f): Form<NewProject>,
) -> Result<Response, PageError> {
    match super::create(&state.db, &user, &f.name, &f.slug).await {
        Ok(p) => Ok(Redirect::to(&format!("/projects/{}/monitors", p.slug)).into_response()),
        Err(ProjectError::Other(e)) => Err(e.into()),
        Err(e) => {
            let page = home_view(&state, &user, f.name, f.slug, e.to_string()).await?;
            Ok((StatusCode::UNPROCESSABLE_ENTITY, render(&page)).into_response())
        }
    }
}

async fn open(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
) -> Result<Response, PageError> {
    let p = super::find_for(&state.db, &user, &slug).await?;
    Ok(Redirect::to(&format!("/projects/{}/monitors", p.slug)).into_response())
}
