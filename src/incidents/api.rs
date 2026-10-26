//! Incidents as JSON for the frontend. Contract: docs/api-specs/05-incidents.md.

use axum::{
    Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, put},
};
use serde::{Deserialize, Serialize};

use super::{IncidentRow, UpdateError, UpdateRow};
use crate::{
    accounts::CurrentUser,
    app::AppState,
    platform::{
        now,
        ui::{Body, UiError, UiResult, ViewerOffset, created, data},
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
        .route("/incidents", get(list).post(create))
        .route("/incident-items", get(items))
        .route("/incidents/{id}", get(detail))
        .route("/incidents/{id}/updates", post(post_update))
        .route("/incidents/{id}/class", put(reclassify))
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct ClassForm {
    /// `planned`, `false_alarm`, or `counted` to restore.
    class: String,
}

/// v1.1 Q46: reclassify an incident as planned work or a false alarm, or count it again. 204.
async fn reclassify(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, id)): Path<(String, i64)>,
    Body(f): Body<ClassForm>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    let class = super::Class::parse(f.class.trim()).ok_or_else(|| {
        UiError::field(
            "class",
            "Pick planned work, a false alarm, or count it again.",
        )
    })?;
    let by = match &user.key {
        Some(k) => format!("API key {}", k.name),
        None => user.email.clone(),
    };
    let author = author(&user);
    if super::reclassify(
        &state.db,
        &state.config.base_url,
        p.id,
        id,
        class,
        &by,
        &author,
        now(),
    )
    .await?
    {
        Ok(StatusCode::NO_CONTENT.into_response())
    } else {
        Err(UiError::NotFound)
    }
}

/// A key writes as itself, so its updates never read as Monita's own.
fn author(user: &crate::accounts::User) -> super::Author {
    match &user.key {
        Some(k) => super::Author::Key(k.name.clone()),
        None => super::Author::User(user.id),
    }
}

/// The form field an update or manual-incident error belongs to; `None` is the whole form.
fn field_of(e: &UpdateError) -> Option<&'static str> {
    use UpdateError::*;
    match e {
        EmptyTitle | TitleTooLong => Some("title"),
        NoItems => Some("items"),
        BadSeverity => Some("severity"),
        EmptyMessage | MessageTooLong => Some("message"),
        BadLabel | Closed => Some("label"),
        TooManyUpdates | TooManyOpen | Other(_) => None,
    }
}

impl From<UpdateError> for UiError {
    fn from(e: UpdateError) -> Self {
        match (field_of(&e), e) {
            (_, UpdateError::Other(e)) => UiError::Internal(e),
            (Some(f), e) => UiError::field(f, e.to_string()),
            (None, e) => UiError::invalid(e.to_string()),
        }
    }
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct ListQuery {
    /// `resolved` lists resolved incidents, `all` lists both; anything else lists open ones.
    state: String,
    /// `YYYY-MM-DD` on the viewer's clock: incidents open at some time in these days.
    from: String,
    to: String,
}

async fn list(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    ViewerOffset(off): ViewerOffset,
    Path(slug): Path<String>,
    Query(q): Query<ListQuery>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    let open = match q.state.as_str() {
        "resolved" => Some(false),
        "all" => None,
        _ => Some(true),
    };
    let range = crate::platform::time::day_range(&q.from, &q.to, off);
    let incidents = super::list(&state.db, p.id, open, range).await?;
    Ok(data(serde_json::json!({ "incidents": incidents })))
}

#[derive(Serialize)]
struct Item {
    /// `monitor:<id>` or `heartbeat:<id>`, as the create body names it.
    value: String,
    kind: &'static str,
    name: String,
    /// `down` or `issue` as the team's pages show it; `None` when it is up.
    state: Option<&'static str>,
}

/// The monitors and heartbeats a manual incident may name, with what is already broken.
async fn items(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    let states = super::team_states_for(&state.db, p.id).await?;
    let items: Vec<Item> = super::item_names(&state.db, p.id)
        .await?
        .into_iter()
        .map(|(r, name)| Item {
            value: format!("{}:{}", r.kind.as_str(), r.id),
            kind: r.kind.as_str(),
            name,
            state: match states.get(&(r.kind, r.id)) {
                Some(super::ItemState::Down) => Some("down"),
                Some(super::ItemState::Issue) => Some("issue"),
                Some(super::ItemState::Maintenance) => Some("maintenance"),
                Some(super::ItemState::Up) | None => None,
            },
        })
        .collect();
    Ok(data(items))
}

#[derive(Serialize)]
struct DetailView {
    incident: IncidentRow,
    /// Names of the affected items; a deleted one shows as "Deleted monitor".
    items: Vec<String>,
    /// Oldest first.
    updates: Vec<UpdateRow>,
    /// The labels the update dialog may offer now.
    labels: &'static [&'static str],
}

async fn detail(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, id)): Path<(String, i64)>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    let (incident, refs, updates) = super::find(&state.db, p.id, id)
        .await?
        .ok_or(UiError::NotFound)?;
    let all = super::item_names(&state.db, p.id).await?;
    let items = refs
        .iter()
        .map(|it| {
            all.iter()
                .find(|(r, _)| r == it)
                .map(|(_, n)| n.clone())
                .unwrap_or_else(|| format!("Deleted {}", it.kind.as_str()))
        })
        .collect();
    let labels = super::allowed_labels(&incident.origin, incident.resolved_at.is_some());
    Ok(data(DetailView {
        incident,
        items,
        updates,
        labels,
    }))
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct UpdateForm {
    label: String,
    message: String,
}

/// US-19. 204 when stored; a refused update is 422 on its field.
async fn post_update(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, id)): Path<(String, i64)>,
    Body(f): Body<UpdateForm>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    match super::post_update(
        &state.db,
        &state.config.base_url,
        p.id,
        id,
        &f.label,
        &f.message,
        &author(&user),
        now(),
    )
    .await
    {
        Ok(true) => Ok(StatusCode::NO_CONTENT.into_response()),
        Ok(false) => Err(UiError::NotFound),
        Err(e) => Err(e.into()),
    }
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct NewForm {
    title: String,
    items: Vec<String>,
    severity: String,
    message: String,
}

/// US-20. 201 with the new incident's id.
async fn create(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
    Body(f): Body<NewForm>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    let incident = super::NewIncident {
        project_id: p.id,
        title: f.title,
        items: f.items,
        severity: f.severity,
        message: f.message,
        author: author(&user),
    };
    let id = super::open_manual(&state.db, &incident, now()).await?;
    Ok(created(serde_json::json!({ "id": id })))
}
