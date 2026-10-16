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

use super::{ApiKeyRow, Project, ProjectError};
use crate::{
    accounts::{AdminUser, CurrentUser, User, UserRow},
    app::AppState,
    platform::{
        now,
        page::{PageError, ProjectNav, Shell},
        render::render,
        time,
    },
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(home))
        .route("/projects", post(create))
        .route("/projects/{slug}", get(open))
        .route("/projects/{slug}/settings", get(settings))
        .route("/projects/{slug}/owner", post(reassign))
        .route("/projects/{slug}/keys", post(create_key))
        .route("/projects/{slug}/keys/{id}/revoke", post(revoke_key))
        .route("/projects/{slug}/allowlist", post(allowlist))
}

pub fn shell(user: &User, active: &'static str, project: Option<&Project>) -> Shell {
    Shell {
        email: user.email.clone(),
        is_admin: user.is_admin(),
        active,
        project: project.map(|p| ProjectNav {
            name: p.name.clone(),
            slug: p.slug.clone(),
        }),
    }
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

async fn home_view(
    state: &AppState,
    user: &User,
    name: String,
    slug: String,
    error: String,
) -> Result<Home, PageError> {
    Ok(Home {
        shell: shell(user, "projects", None),
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

#[derive(Template)]
#[template(path = "projects/settings.html")]
struct Settings {
    shell: Shell,
    project: Project,
    keys: Vec<ApiKeyRow>,
    tls: Vec<UserRow>,
    new_key: Option<String>,
    key_name: String,
    key_error: String,
    allowlist: String,
    allowlist_error: String,
    now: i64,
}

impl Settings {
    fn ago(&self, ts: &Option<i64>) -> String {
        ts.map(|t| time::ago(t, self.now))
            .unwrap_or_else(|| "never".into())
    }
    fn utc(&self, ts: &i64) -> String {
        time::utc(*ts)
    }
}

async fn settings_view(
    state: &AppState,
    user: &User,
    project: Project,
) -> Result<Settings, PageError> {
    let keys = super::list_keys(&state.db, project.id).await?;
    let tls = if user.is_admin() {
        crate::accounts::list_tls(&state.db).await?
    } else {
        Vec::new()
    };
    Ok(Settings {
        shell: shell(user, "settings", Some(&project)),
        allowlist: project
            .ip_allowlist
            .clone()
            .unwrap_or_default()
            .replace(',', "\n"),
        project,
        keys,
        tls,
        new_key: None,
        key_name: String::new(),
        key_error: String::new(),
        allowlist_error: String::new(),
        now: now(),
    })
}

async fn settings(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
) -> Result<Response, PageError> {
    let p = super::find_for(&state.db, &user, &slug).await?;
    Ok(render(&settings_view(&state, &user, p).await?))
}

#[derive(Deserialize)]
struct OwnerForm {
    owner_id: i64,
}

async fn reassign(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
    Path(slug): Path<String>,
    Form(f): Form<OwnerForm>,
) -> Result<Response, PageError> {
    let p = super::find_for(&state.db, &admin, &slug).await?;
    let valid_owner = f.owner_id == admin.id
        || crate::accounts::list_tls(&state.db)
            .await?
            .iter()
            .any(|u| u.id == f.owner_id);
    if !valid_owner {
        return Err(PageError::NotFound);
    }
    super::reassign(&state.db, p.id, f.owner_id).await?;
    Ok(Redirect::to(&format!("/projects/{slug}/settings")).into_response())
}

#[derive(Deserialize)]
struct KeyForm {
    name: String,
}

async fn create_key(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
    Form(f): Form<KeyForm>,
) -> Result<Response, PageError> {
    let p = super::find_for(&state.db, &user, &slug).await?;
    let project_id = p.id;
    let mut page = settings_view(&state, &user, p).await?;
    match super::create_key(&state.db, project_id, &f.name).await {
        Ok(raw) => {
            page.keys = super::list_keys(&state.db, project_id).await?;
            page.new_key = Some(raw);
            Ok(render(&page))
        }
        Err(ProjectError::Other(e)) => Err(e.into()),
        Err(e) => {
            page.key_name = f.name;
            page.key_error = e.to_string();
            Ok((StatusCode::UNPROCESSABLE_ENTITY, render(&page)).into_response())
        }
    }
}

async fn revoke_key(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, id)): Path<(String, i64)>,
) -> Result<Response, PageError> {
    let p = super::find_for(&state.db, &user, &slug).await?;
    super::revoke_key(&state.db, p.id, id).await?;
    Ok(Redirect::to(&format!("/projects/{slug}/settings")).into_response())
}

#[derive(Deserialize)]
struct AllowlistForm {
    cidrs: String,
}

async fn allowlist(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
    Form(f): Form<AllowlistForm>,
) -> Result<Response, PageError> {
    let p = super::find_for(&state.db, &user, &slug).await?;
    let project_id = p.id;
    match super::set_allowlist(&state.db, project_id, &f.cidrs).await {
        Ok(()) => Ok(Redirect::to(&format!("/projects/{slug}/settings")).into_response()),
        Err(ProjectError::Other(e)) => Err(e.into()),
        Err(e) => {
            let mut page = settings_view(&state, &user, p).await?;
            page.allowlist = f.cidrs;
            page.allowlist_error = e.to_string();
            Ok((StatusCode::UNPROCESSABLE_ENTITY, render(&page)).into_response())
        }
    }
}
