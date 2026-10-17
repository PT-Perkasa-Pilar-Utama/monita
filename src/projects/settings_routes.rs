//! A project's settings: General, API keys and Allowlist, each its own page under one sidebar.
//! Alerts is in `notify`. Contract: docs/api-specs/03-projects.md.

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
        page::{PageError, Shell},
        render::render,
        time,
    },
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/projects/{slug}/settings", get(general))
        .route("/projects/{slug}/settings/keys", get(keys))
        .route("/projects/{slug}/settings/allowlist", get(allowlist_page))
        .route("/projects/{slug}/owner", post(reassign))
        .route("/projects/{slug}/keys", post(create_key))
        .route("/projects/{slug}/keys/{id}/revoke", post(revoke_key))
        .route("/projects/{slug}/allowlist", post(allowlist))
}

#[derive(Template)]
#[template(path = "projects/settings.html")]
struct General {
    shell: Shell,
    project: Project,
    /// Who `reassign` accepts: the acting super admin first, then every active TL.
    owners: Vec<UserRow>,
    /// The Reassign dialog's field after a rejected try; the current owner before.
    owner_email: String,
    error: String,
}

impl General {
    fn utc(&self, ts: &i64) -> String {
        time::utc(*ts)
    }
    fn owner_value(&self) -> &str {
        match self.owner_email.trim() {
            "" => &self.project.owner_email,
            v => v,
        }
    }
}

async fn general_view(
    state: &AppState,
    user: &User,
    project: Project,
) -> Result<General, PageError> {
    let owners = if user.is_admin() {
        super::eligible_owners(&state.db, user).await?
    } else {
        Vec::new()
    };
    Ok(General {
        shell: super::shell(&state.db, user, "settings", Some(&project)).await?,
        project,
        owners,
        owner_email: String::new(),
        error: String::new(),
    })
}

async fn general(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
) -> Result<Response, PageError> {
    let project = super::find_for(&state.db, &user, &slug).await?;
    Ok(render(&general_view(&state, &user, project).await?))
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct OwnerForm {
    owner_email: String,
}

async fn reassign(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
    Path(slug): Path<String>,
    Form(f): Form<OwnerForm>,
) -> Result<Response, PageError> {
    let p = super::find_for(&state.db, &admin, &slug).await?;
    let Some(owner_id) = super::find_owner(&state.db, &admin, &f.owner_email).await? else {
        let mut page = general_view(&state, &admin, p).await?;
        page.owner_email = f.owner_email;
        page.error = ProjectError::UnknownOwner.to_string();
        return Ok((StatusCode::UNPROCESSABLE_ENTITY, render(&page)).into_response());
    };
    super::reassign(&state.db, p.id, owner_id, &admin).await?;
    Ok(Redirect::to(&format!("/projects/{slug}/settings")).into_response())
}

#[derive(Template)]
#[template(path = "projects/keys.html")]
struct Keys {
    shell: Shell,
    project: Project,
    keys: Vec<ApiKeyRow>,
    /// The raw key, shown once right after it is created.
    new_key: Option<String>,
    name: String,
    error: String,
    now: i64,
}

impl Keys {
    fn ago(&self, ts: &Option<i64>) -> String {
        ts.map(|t| time::ago(t, self.now))
            .unwrap_or_else(|| "never".into())
    }
}

async fn keys_view(state: &AppState, user: &User, project: Project) -> Result<Keys, PageError> {
    Ok(Keys {
        shell: super::shell(&state.db, user, "keys", Some(&project)).await?,
        keys: super::list_keys(&state.db, project.id).await?,
        project,
        new_key: None,
        name: String::new(),
        error: String::new(),
        now: now(),
    })
}

async fn keys(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
) -> Result<Response, PageError> {
    let p = super::find_for(&state.db, &user, &slug).await?;
    Ok(render(&keys_view(&state, &user, p).await?))
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
    let created = super::create_key(&state.db, p.id, &f.name).await;
    let mut page = keys_view(&state, &user, p).await?;
    match created {
        Ok(raw) => {
            page.new_key = Some(raw);
            Ok(render(&page))
        }
        Err(ProjectError::Other(e)) => Err(e.into()),
        Err(e) => {
            page.name = f.name;
            page.error = e.to_string();
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
    Ok(Redirect::to(&format!("/projects/{slug}/settings/keys")).into_response())
}

#[derive(Template)]
#[template(path = "projects/allowlist.html")]
struct Allowlist {
    shell: Shell,
    project: Project,
    /// One CIDR per line.
    cidrs: String,
    error: String,
}

async fn allowlist_view(
    state: &AppState,
    user: &User,
    project: Project,
) -> Result<Allowlist, PageError> {
    Ok(Allowlist {
        shell: super::shell(&state.db, user, "allowlist", Some(&project)).await?,
        cidrs: project
            .ip_allowlist
            .clone()
            .unwrap_or_default()
            .replace(',', "\n"),
        project,
        error: String::new(),
    })
}

async fn allowlist_page(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
) -> Result<Response, PageError> {
    let p = super::find_for(&state.db, &user, &slug).await?;
    Ok(render(&allowlist_view(&state, &user, p).await?))
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
    match super::set_allowlist(&state.db, p.id, &f.cidrs).await {
        Ok(()) => Ok(Redirect::to(&format!("/projects/{slug}/settings/allowlist")).into_response()),
        Err(ProjectError::Other(e)) => Err(e.into()),
        Err(e) => {
            let mut page = allowlist_view(&state, &user, p).await?;
            page.cidrs = f.cidrs;
            page.error = e.to_string();
            Ok((StatusCode::UNPROCESSABLE_ENTITY, render(&page)).into_response())
        }
    }
}
