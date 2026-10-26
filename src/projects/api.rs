//! Projects as JSON for the frontend. Contract: docs/api-specs/03-projects.md.

use axum::response::IntoResponse;
use axum::{
    Router,
    extract::{Path, Query, State},
    routing::get,
};
use serde::{Deserialize, Serialize};

use super::{Project, ProjectError, ProjectInput, TabCounts};
use crate::{
    accounts::{AdminUser, CurrentUser},
    app::AppState,
    platform::ui::{Body, UiError, UiResult, ViewerOffset, created, data},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/ui/projects", get(list).post(create))
        // Not under /projects/: a project may be called `chart`.
        .route("/api/ui/projects-chart", get(chart))
        .route("/api/ui/projects/{slug}", get(detail).delete(delete))
        .route("/api/ui/projects/{slug}/overview", get(overview))
        .merge(super::settings_api::router())
}

/// The project's own reads that a project API key reaches under `/api/v1/projects/{slug}`
/// (v1.1 Q44). Delete, profile, keys, the allowlist and secrets stay off it.
pub fn key_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(detail))
        .route("/overview", get(overview))
}

/// A project as every page shows it.
#[derive(Serialize)]
pub struct ProjectView {
    pub name: String,
    pub slug: String,
    pub owner_email: String,
    /// For the owner's picture at `/users/{owner_id}/avatar`.
    pub owner_id: i64,
    pub owner_avatar_at: Option<i64>,
    pub description: Option<String>,
    /// Versioned by `icon_at`, so a new icon is a new URL. `None` shows the first letter.
    pub icon_url: Option<String>,
    pub ip_allowlist: Option<String>,
    pub created_at: i64,
    pub pages: super::PublicPages,
    pub homepage_url: Option<String>,
}

impl From<&Project> for ProjectView {
    fn from(p: &Project) -> Self {
        ProjectView {
            name: p.name.clone(),
            slug: p.slug.clone(),
            owner_email: p.owner_email.clone(),
            owner_id: p.owner_id,
            owner_avatar_at: p.owner_avatar_at,
            description: p.description.clone(),
            icon_url: p
                .icon_at
                .map(|v| format!("/projects/{}/icon?v={v}", p.slug)),
            ip_allowlist: p.ip_allowlist.clone(),
            created_at: p.created_at,
            pages: p.pages,
            homepage_url: p.homepage_url.clone(),
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
    projects: Vec<ProjectRow>,
    /// Projects that match the search, on every page.
    total: i64,
    page: i64,
    page_size: i64,
    /// Empty for a tech lead, who always owns what they create.
    owners: Vec<OwnerView>,
    /// The owner filter's choices: who owns a project. Empty for a tech lead.
    owner_options: Vec<String>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct ListQuery {
    /// Counts from 1; a missing or bad value reads page 1.
    page: String,
    q: String,
    /// An owner's email; empty for any.
    owner: String,
    /// `name` or `oldest`; anything else is newest first.
    sort: String,
    /// `down`, `issue` or `up`; anything else is every state.
    state: String,
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

/// The page the list's query asks for.
async fn page_of(
    state: &AppState,
    user: &crate::accounts::User,
    q: &ListQuery,
) -> anyhow::Result<super::ProjectPage> {
    let sort = match q.sort.as_str() {
        "name" => super::Sort::Name,
        "oldest" => super::Sort::Oldest,
        _ => super::Sort::Newest,
    };
    super::page_for(
        &state.db,
        user,
        &q.q,
        &q.owner,
        sort,
        super::StateFilter::parse(&q.state),
        q.page.trim().parse().unwrap_or(1),
    )
    .await
}

/// The chart of the same page of projects (v1.1 Q39): a line each for its monitors' response time.
async fn chart(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Query(q): Query<ListQuery>,
    ViewerOffset(off): ViewerOffset,
) -> UiResult {
    let found = page_of(&state, &user, &q).await?;
    let projects: Vec<(i64, String)> = found
        .projects
        .iter()
        .map(|p| (p.id, p.name.clone()))
        .collect();
    let chart =
        crate::checks::series::projects(&state.db, &projects, crate::platform::now(), off).await?;
    // The page links by slug; the chart names projects by id.
    let slugs: std::collections::HashMap<i64, &str> = found
        .projects
        .iter()
        .map(|p| (p.id, p.slug.as_str()))
        .collect();
    Ok(data(serde_json::json!({ "chart": chart, "slugs": slugs })))
}

/// One page: a tech lead's own projects, or every project for a super admin.
async fn list(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Query(q): Query<ListQuery>,
    ViewerOffset(off): ViewerOffset,
) -> UiResult {
    let found = page_of(&state, &user, &q).await?;
    let owner_options = if user.is_admin() {
        super::owner_emails(&state.db).await?
    } else {
        Vec::new()
    };
    Ok(data(ProjectList {
        projects: rows(&state, &found.projects, off).await?,
        total: found.total,
        page: found.page,
        page_size: crate::platform::caps::PROJECTS_PAGE,
        owners: owners(&state, &user).await?,
        owner_options,
    }))
}

/// A project in the list, with how it is doing right now.
#[derive(Serialize)]
struct ProjectRow {
    #[serde(flatten)]
    project: ProjectView,
    pulse: super::Pulse,
}

/// One page of projects, `caps::PROJECTS_PAGE` at most, each with its pulse.
// ponytail: one pulse read per project, 20 a page; batch them into grouped queries if the list gets slow.
async fn rows(state: &AppState, projects: &[Project], off: i64) -> anyhow::Result<Vec<ProjectRow>> {
    let mut out = Vec::with_capacity(projects.len());
    for p in projects {
        out.push(ProjectRow {
            project: ProjectView::from(p),
            pulse: super::pulse(&state.db, p.id, off).await?,
        });
    }
    Ok(out)
}

/// A form error, on the field it concerns.
pub fn form_error(e: ProjectError) -> UiError {
    let field = match &e {
        ProjectError::InvalidName => "name",
        ProjectError::UnknownOwner => "owner_email",
        ProjectError::LongDescription => "description",
        ProjectError::BadIcon(_) => "icon",
        ProjectError::InvalidCidr(_) => "cidrs",
        ProjectError::ConfirmMismatch => "confirm",
        ProjectError::Full(_)
        | ProjectError::Other(_)
        | ProjectError::Invalid(_)
        | ProjectError::CannotOwn => "",
    };
    match e {
        ProjectError::Other(e) => UiError::Internal(e),
        ProjectError::CannotOwn => UiError::Forbidden,
        ProjectError::Invalid(bad) => UiError::fields(
            bad.into_iter()
                .map(|(f, m)| (f.to_string(), m.to_string()))
                .collect(),
        ),
        e if field.is_empty() => UiError::invalid(e.to_string()),
        e => UiError::field(field, e.to_string()),
    }
}

async fn create(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Body(input): Body<ProjectInput>,
) -> UiResult {
    let p = super::create(&state.db, &user, &input)
        .await
        .map_err(form_error)?;
    Ok(created(ProjectView::from(&p)))
}

/// What every page inside a project needs: the project and the numbers beside its tabs.
#[derive(Serialize)]
struct ProjectHead {
    project: ProjectView,
    counts: TabCounts,
}

async fn detail(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
) -> UiResult {
    let p = super::find_for(&state.db, &user, &slug).await?;
    Ok(data(ProjectHead {
        counts: super::tab_counts(&state.db, p.id).await?,
        project: ProjectView::from(&p),
    }))
}

async fn overview(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
    ViewerOffset(off): ViewerOffset,
) -> UiResult {
    let p = super::find_for(&state.db, &user, &slug).await?;
    Ok(data(super::overview::read(&state.db, p.id, off).await?))
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct DeleteInput {
    /// The project's name, typed again.
    confirm: String,
}

/// Super admin only: deletes the project and everything in it. 204.
async fn delete(
    State(state): State<AppState>,
    AdminUser(user): AdminUser,
    Path(slug): Path<String>,
    Body(f): Body<DeleteInput>,
) -> UiResult {
    let p = super::find_for(&state.db, &user, &slug).await?;
    super::delete(&state.db, &p, &f.confirm)
        .await
        .map_err(form_error)?;
    Ok(axum::http::StatusCode::NO_CONTENT.into_response())
}
