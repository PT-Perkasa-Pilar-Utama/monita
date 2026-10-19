//! Changelog JSON: the team's list and form under the project, and the public index and release
//! under `/api/ui/public/`. Public answers carry only what the team typed: version, day, title
//! and the changes.

use axum::{
    Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use serde::Deserialize;

use super::{ChangelogError, ReleaseInput};
use crate::{
    accounts::CurrentUser,
    app::AppState,
    platform::{
        caps,
        ui::{Body, UiError, UiResult, created, data, public_cache},
    },
    projects,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/ui/projects/{slug}/changelog", get(list).post(create))
        .route(
            "/api/ui/projects/{slug}/changelog/{id}",
            axum::routing::put(update).delete(delete),
        )
        .route("/api/ui/public/{slug}/changelog", get(public_index))
        .route("/api/ui/public/{slug}/changelog/{version}", get(public_one))
}

impl From<ChangelogError> for UiError {
    fn from(e: ChangelogError) -> Self {
        match e {
            ChangelogError::Invalid(bad) => UiError::fields(
                bad.into_iter()
                    .map(|(f, m)| (f.to_string(), m.to_string()))
                    .collect(),
            ),
            ChangelogError::Taken => UiError::field("version", e.to_string()),
            ChangelogError::NotFound => UiError::NotFound,
            ChangelogError::Other(e) => UiError::Internal(e),
        }
    }
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct PageQuery {
    page: String,
}

async fn list(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
    Query(q): Query<PageQuery>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    let page = q.page.trim().parse().unwrap_or(1).max(1);
    let (releases, total) = super::page(&state.db, p.id, page).await?;
    Ok(data(serde_json::json!({
        "releases": releases, "total": total, "page": page, "page_size": caps::CHANGELOG_PAGE,
    })))
}

async fn create(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
    Body(f): Body<ReleaseInput>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    let id = super::create(&state.db, p.id, &f).await?;
    Ok(created(serde_json::json!({ "id": id })))
}

async fn update(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, id)): Path<(String, i64)>,
    Body(f): Body<ReleaseInput>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    super::update(&state.db, p.id, id, &f).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn delete(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, id)): Path<(String, i64)>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    super::delete(&state.db, p.id, id).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

/// The project when its Changelog tab is on; not found otherwise.
async fn public_project(state: &AppState, slug: &str) -> Result<projects::Project, UiError> {
    projects::find_public(&state.db, slug)
        .await?
        .filter(|p| p.pages.changelog)
        .ok_or(UiError::NotFound)
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Before {
    /// `<released_on>.<id>` from `older`; a bad value reads the first page.
    before: String,
}

async fn public_index(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(q): Query<Before>,
) -> UiResult {
    let p = public_project(&state, &slug).await?;
    let cursor = q
        .before
        .split_once('.')
        .and_then(|(d, id)| Some((d.to_string(), id.parse::<i64>().ok()?)));
    let releases = super::index(&state.db, p.id, cursor).await?;
    let older = (releases.len() as i64 == caps::CHANGELOG_PAGE)
        .then(|| {
            releases
                .last()
                .map(|r| format!("{}.{}", r.released_on, r.id))
        })
        .flatten();
    Ok(public_cache(data(
        serde_json::json!({ "releases": releases, "older": older }),
    )))
}

/// One release by version; `latest` is the newest.
async fn public_one(
    State(state): State<AppState>,
    Path((slug, version)): Path<(String, String)>,
) -> UiResult {
    let p = public_project(&state, &slug).await?;
    let want = (version != "latest").then_some(version.as_str());
    let (release, newer, older) = super::one(&state.db, p.id, want)
        .await?
        .ok_or(UiError::NotFound)?;
    let body = serde_json::json!({
        "release": { "version": release.version, "released_on": release.released_on, "title": release.title, "sections": release.sections },
        "newer": newer,
        "older": older,
    });
    Ok(public_cache(data(body)))
}
