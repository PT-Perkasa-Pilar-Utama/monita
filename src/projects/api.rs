//! Projects as JSON for the frontend. Contract: docs/api-specs/03-projects.md.

use axum::{Json, Router, extract::State, routing::get};
use serde::Serialize;

use super::{Project, ProjectError, ProjectInput};
use crate::{
    accounts::CurrentUser,
    app::AppState,
    platform::ui::{UiError, UiResult, created, data},
};

pub fn router() -> Router<AppState> {
    Router::new().route("/api/ui/projects", get(list).post(create))
}

/// A project as every page shows it.
#[derive(Serialize)]
pub struct ProjectView {
    pub name: String,
    pub slug: String,
    pub owner_email: String,
    pub description: Option<String>,
    /// Versioned by `icon_at`, so a new icon is a new URL. `None` shows the first letter.
    pub icon_url: Option<String>,
    pub ip_allowlist: Option<String>,
    pub created_at: i64,
}

impl From<&Project> for ProjectView {
    fn from(p: &Project) -> Self {
        ProjectView {
            name: p.name.clone(),
            slug: p.slug.clone(),
            owner_email: p.owner_email.clone(),
            description: p.description.clone(),
            icon_url: p
                .icon_at
                .map(|v| format!("/projects/{}/icon?v={v}", p.slug)),
            ip_allowlist: p.ip_allowlist.clone(),
            created_at: p.created_at,
        }
    }
}

/// Someone a super admin may make an owner.
#[derive(Serialize)]
pub struct OwnerView {
    pub email: String,
    pub role: String,
}

#[derive(Serialize)]
struct ProjectList {
    projects: Vec<ProjectView>,
    /// Empty for a tech lead, who always owns what they create.
    owners: Vec<OwnerView>,
}

pub async fn owners(
    state: &AppState,
    user: &crate::accounts::User,
) -> anyhow::Result<Vec<OwnerView>> {
    if !user.is_admin() {
        return Ok(Vec::new());
    }
    Ok(super::eligible_owners(&state.db, user)
        .await?
        .into_iter()
        .map(|u| OwnerView {
            email: u.email,
            role: u.role,
        })
        .collect())
}

/// A tech lead gets their own projects; a super admin gets every project.
async fn list(State(state): State<AppState>, CurrentUser(user): CurrentUser) -> UiResult {
    let projects = super::list_for(&state.db, &user).await?;
    Ok(data(ProjectList {
        projects: projects.iter().map(ProjectView::from).collect(),
        owners: owners(&state, &user).await?,
    }))
}

/// A form error, on the field it concerns.
pub fn form_error(e: ProjectError) -> UiError {
    let field = match &e {
        ProjectError::InvalidName => "name",
        ProjectError::UnknownOwner => "owner_email",
        ProjectError::LongDescription => "description",
        ProjectError::BadIcon(_) => "icon",
        ProjectError::InvalidCidr(_) => "cidrs",
        ProjectError::Full(_) | ProjectError::Other(_) => "",
    };
    match e {
        ProjectError::Other(e) => UiError::Internal(e),
        e if field.is_empty() => UiError::invalid(e.to_string()),
        e => UiError::field(field, e.to_string()),
    }
}

async fn create(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(input): Json<ProjectInput>,
) -> UiResult {
    let p = super::create(&state.db, &user, &input)
        .await
        .map_err(form_error)?;
    Ok(created(ProjectView::from(&p)))
}
