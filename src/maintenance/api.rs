//! Maintenance windows as JSON for the team. Contract: docs/api-specs/05-incidents.md section 5.3.
//! The public status page reads active windows through `status`, with its own wire type.

use axum::{
    Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};

use super::{MaintenanceError, State as WindowState, WindowInput};
use crate::{
    accounts::CurrentUser,
    app::AppState,
    incidents::SourceKind,
    platform::{
        caps, now,
        ui::{Body, UiError, UiResult, created, data},
    },
    projects,
};

pub fn router() -> Router<AppState> {
    Router::new().nest("/api/ui/projects/{slug}", project_routes())
}

/// The routes below `/api/ui/projects/{slug}` that a project API key also reaches under
/// `/api/v1/projects/{slug}` (v1.1 Q44).
pub fn project_routes() -> Router<AppState> {
    Router::new()
        .route("/maintenance", get(list).post(create))
        .route("/maintenance/{id}", get(one).put(update).delete(remove))
        .route("/maintenance/{id}/end", post(end))
}

impl From<MaintenanceError> for UiError {
    fn from(e: MaintenanceError) -> Self {
        match e {
            MaintenanceError::Invalid(bad) => UiError::fields(
                bad.into_iter()
                    .map(|(f, m)| (f.to_string(), m.to_string()))
                    .collect(),
            ),
            MaintenanceError::NotFound => UiError::NotFound,
            MaintenanceError::Other(e) => UiError::Internal(e),
            MaintenanceError::Full
            | MaintenanceError::Started
            | MaintenanceError::NotActive
            | MaintenanceError::Ended => UiError::invalid(e.to_string()),
        }
    }
}

#[derive(Serialize)]
struct Item {
    /// `kind:id`, as the form sends it.
    value: String,
    kind: &'static str,
    name: String,
}

#[derive(Serialize)]
struct WindowView {
    id: i64,
    title: String,
    message: String,
    starts_at: i64,
    ends_at: i64,
    state: WindowState,
    /// The covered items that still exist.
    items: Vec<Item>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct PageQuery {
    page: String,
    q: String,
    state: String,
    sort: String,
}

async fn list(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
    Query(q): Query<PageQuery>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    let page = q.page.trim().parse().unwrap_or(1).max(1);
    let t = now();
    let filter = super::Filter {
        q: q.q,
        state: super::State::parse(&q.state),
        oldest: q.sort == "oldest",
    };
    let (windows, total) = super::page(&state.db, p.id, page, &filter, t).await?;
    let names = crate::checks::source_names(&state.db.read().await?, p.id).await?;
    let windows: Vec<WindowView> = windows.into_iter().map(|w| view(w, &names, t)).collect();
    Ok(data(serde_json::json!({
        "windows": windows, "total": total, "page": page, "page_size": caps::WINDOWS_PAGE,
    })))
}

/// A window as the list and the edit form show it, with the names of the items that still exist.
fn view(w: super::Window, names: &[(SourceKind, i64, String)], t: i64) -> WindowView {
    WindowView {
        state: super::state(w.starts_at, w.ends_at, t),
        items: w
            .items
            .iter()
            .filter_map(|r| {
                let (kind, id, name) = names.iter().find(|(k, i, _)| *k == r.kind && *i == r.id)?;
                Some(Item {
                    value: format!("{}:{id}", kind.as_str()),
                    kind: kind.as_str(),
                    name: name.clone(),
                })
            })
            .collect(),
        id: w.id,
        title: w.title,
        message: w.message,
        starts_at: w.starts_at,
        ends_at: w.ends_at,
    }
}

async fn one(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, id)): Path<(String, i64)>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    let w = super::one(&state.db, p.id, id)
        .await?
        .ok_or(UiError::NotFound)?;
    let names = crate::checks::source_names(&state.db.read().await?, p.id).await?;
    Ok(data(view(w, &names, now())))
}

/// Q50: a scheduled window changes every field, an active one its title, message and end.
async fn update(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, id)): Path<(String, i64)>,
    Body(f): Body<WindowInput>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    super::update(&state.db, p.id, id, &f, now()).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn create(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
    Body(f): Body<WindowInput>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    let id = super::create(&state.db, p.id, &f, now()).await?;
    Ok(created(serde_json::json!({ "id": id })))
}

async fn remove(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, id)): Path<(String, i64)>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    super::delete(&state.db, p.id, id, now()).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn end(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, id)): Path<(String, i64)>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    super::end_now(&state.db, p.id, id, now()).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
