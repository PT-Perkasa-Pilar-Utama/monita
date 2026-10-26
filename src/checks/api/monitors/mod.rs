//! Monitors: the list with its sparklines, one monitor's page, and the form's writes.

use std::collections::HashMap;

use axum::{
    Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use serde::Serialize;

mod form;
mod view;

use super::{Paging, invalid, views};
use crate::{
    accounts::CurrentUser,
    app::AppState,
    checks::{
        self, CheckRow, Monitor, MonitorInput, Saved,
        insight::Insight,
        overview::{self, Day, check_summary, interval_label},
    },
    incidents::{self, SourceKind},
    platform::{
        caps, now,
        ui::{Body, UiError, UiResult, created, data},
    },
    projects,
};
use form::MonitorForm;
use view::{MonitorView, view};

/// Checks per page of a monitor's history.
const PAGE: i64 = 25;

/// The routes below `/api/ui/projects/{slug}` that a project API key also reaches under
/// `/api/v1/projects/{slug}` (v1.1 Q44).
pub fn project_routes() -> Router<AppState> {
    Router::new()
        .route("/monitors", get(list).post(create))
        .route("/monitors/{id}", get(detail).put(update).delete(delete))
        .route("/monitors/{id}/paused", axum::routing::put(paused))
}

#[derive(Serialize)]
struct Row {
    #[serde(flatten)]
    monitor: MonitorView,
    /// The last 24 hours: sparkline path, failed hours, uptime and mean response.
    day: Option<Day>,
}

#[derive(Serialize)]
struct ListView {
    monitors: Vec<Row>,
    /// The worst state and one sentence about every monitor.
    summary: Summary,
}

#[derive(Serialize)]
struct Summary {
    kind: &'static str,
    text: String,
}

async fn list(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    let monitors = checks::list_monitors(&state.db, p.id).await?;
    let states = incidents::team_states_for(&state.db, p.id).await?;
    let mut days: HashMap<i64, Day> = overview::last_day(&state.db, p.id, now()).await?;
    let state_of = |m: &Monitor| {
        checks::item_state(
            m.last.is_some(),
            m.paused_at.is_some(),
            states.get(&(SourceKind::Monitor, m.id)).copied(),
        )
        .0
    };
    let kinds: Vec<&str> = monitors.iter().map(state_of).collect();
    let (kind, text) = overview::summary(&kinds, "monitor");
    let rows = monitors
        .into_iter()
        .map(|m| {
            let s = state_of(&m);
            Row {
                day: days.remove(&m.id),
                monitor: view(m, s, user.is_read_only()),
            }
        })
        .collect();
    Ok(data(ListView {
        monitors: rows,
        summary: Summary { kind, text },
    }))
}

/// `{paused}`: pause or resume (v1.1 Q38). 204, or 404 for a monitor of another project.
async fn paused(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, id)): Path<(String, i64)>,
    crate::platform::ui::Body(f): crate::platform::ui::Body<super::super::api::PausedInput>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    if !checks::set_paused(&state.db, p.id, SourceKind::Monitor, id, f.paused).await? {
        return Err(UiError::NotFound);
    }
    Ok(axum::http::StatusCode::NO_CONTENT.into_response())
}

#[derive(Serialize)]
struct DetailView {
    monitor: MonitorView,
    stats: Vec<views::StatView>,
    insight: views::InsightView,
    /// The newest checks, `PAGE` at most, before the cursor.
    checks: Vec<CheckRow>,
    /// The cursor for the next older page, when there is one.
    older: Option<String>,
    /// The locations the monitor also checks from.
    location_ids: Vec<i64>,
    /// Each place's newest result, this server first; empty without locations.
    places: Vec<checks::locations::PlaceView>,
    /// The active maintenance window over it, or null.
    maintenance: Option<views::MaintenanceView>,
}

async fn detail(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, id)): Path<(String, i64)>,
    Query(paging): Query<Paging>,
    crate::platform::ui::ViewerOffset(off): crate::platform::ui::ViewerOffset,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    let m = checks::find_monitor(&state.db, p.id, id)
        .await?
        .ok_or(UiError::NotFound)?;
    let rows = checks::recent_checks(&state.db, id, paging.cursor(), PAGE).await?;
    let older = (rows.len() as i64 == PAGE)
        .then(|| rows.last().map(|c| format!("{}.{}", c.checked_at, c.id)))
        .flatten();
    let incident = incidents::team_states_for(&state.db, p.id)
        .await?
        .get(&(SourceKind::Monitor, id))
        .copied();
    let s = checks::item_state(m.last.is_some(), m.paused_at.is_some(), incident).0;
    let insight = Insight::monitor(&state.db, &m, paging.range(), now(), off).await?;
    let last = match &m.last {
        Some(c) => (
            "Last check",
            crate::platform::time::ago(c.checked_at, insight.now),
            check_summary(c),
            if c.ok { "up" } else { "down" },
        ),
        None => (
            "Last check",
            "Never".into(),
            "the first runs soon".into(),
            "plain",
        ),
    };
    let stats = views::stats([
        insight.streak_stat(m.last.is_some()),
        last,
        (
            "Mean response",
            insight.mean(),
            insight.range_word(),
            "plain",
        ),
        insight.incidents_stat(),
    ]);
    let (location_ids, places) = checks::locations::places(&state.db, id).await?;
    let secret_name = match m.auth_secret_id {
        Some(sid) => projects::secrets::name_of(&state.db.read().await?, p.id, sid).await?,
        None => None,
    };
    let maintenance = views::maintenance_of(&state.db, p.id, SourceKind::Monitor, id).await?;
    Ok(data(DetailView {
        maintenance,
        monitor: MonitorView {
            auth_secret_name: secret_name,
            ..view(m, s, user.is_read_only())
        },
        stats,
        insight: insight.into(),
        checks: rows,
        older,
        location_ids,
        places,
    }))
}

/// Field errors under the form's own names: the form sends seconds, `validate` checks milliseconds.
fn form_error(mut errors: checks::FieldErrors) -> UiError {
    if let Some(m) = errors.remove("timeout_ms") {
        errors.insert("timeout_s", m);
    }
    invalid(errors)
}

fn refused_error(s: Saved) -> UiError {
    match s {
        Saved::Full => UiError::field("name", caps::MONITORS_FULL),
        Saved::ForeignSecret => UiError::field("auth_secret_id", "Pick a secret of this project."),
        Saved::UnknownLocation => UiError::field("location_ids", "Pick locations from the list."),
        Saved::Done(_) | Saved::NotFound => UiError::NotFound,
    }
}

/// AC-09.01: 201 with the new monitor's id.
async fn create(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
    Body(f): Body<MonitorForm>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    let input = MonitorInput::from(f);
    let errors = input.validate();
    if !errors.is_empty() {
        return Err(form_error(errors));
    }
    match checks::create_monitor(&state.db, p.id, &input).await? {
        Saved::Done(id) => Ok(created(serde_json::json!({ "id": id }))),
        refused => Err(refused_error(refused)),
    }
}

async fn update(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, id)): Path<(String, i64)>,
    Body(f): Body<MonitorForm>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    let input = MonitorInput::from(f);
    let errors = input.validate();
    if !errors.is_empty() {
        return Err(form_error(errors));
    }
    match checks::update_monitor(&state.db, p.id, id, &input).await? {
        Saved::Done(_) => Ok(StatusCode::NO_CONTENT.into_response()),
        refused => Err(refused_error(refused)),
    }
}

/// The monitor, its checks and summaries go; an open incident on it closes.
async fn delete(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, id)): Path<(String, i64)>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    checks::delete_monitor(&state.db, &state.config.base_url, p.id, id).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

#[cfg(test)]
mod tests {
    use super::view::hide_query;

    #[test]
    fn a_supervisor_sees_the_url_without_its_query() {
        assert_eq!(
            hide_query("https://api.x.co/health?token=abc&x=1"),
            "https://api.x.co/health?(hidden)"
        );
        assert_eq!(
            hide_query("https://api.x.co/#k=1"),
            "https://api.x.co/#(hidden)"
        );
        assert_eq!(
            hide_query("https://api.x.co/health"),
            "https://api.x.co/health"
        );
        assert_eq!(hide_query("db.internal:5432"), "db.internal:5432");
    }
}
