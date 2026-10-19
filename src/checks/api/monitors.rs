//! Monitors: the list with its sparklines, one monitor's page, and the form's writes.

use std::collections::HashMap;

use axum::{
    Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use serde::{Deserialize, Serialize};

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

/// Checks per page of a monitor's history.
const PAGE: i64 = 25;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/ui/projects/{slug}/monitors", get(list).post(create))
        .route(
            "/api/ui/projects/{slug}/monitors/{id}",
            get(detail).put(update).delete(delete),
        )
}

/// A monitor as its rows and pages show it.
#[derive(Serialize)]
struct MonitorView {
    id: i64,
    name: String,
    url: String,
    interval_s: i64,
    /// `30s`, `1m`, `3m` or `5m`.
    every: &'static str,
    timeout_s: i64,
    expected_status: String,
    keyword: Option<String>,
    skip_tls_verify: bool,
    severity: String,
    is_public: bool,
    alert: bool,
    auto_incident: bool,
    fails_to_open: i64,
    method: String,
    /// `Name: value` lines. Owner and super admin only, like the rest of this view.
    headers: String,
    body: Option<String>,
    /// `none`, `bearer`, `basic` or `header`. Never the secret's value.
    auth_kind: String,
    auth_secret_id: Option<i64>,
    /// Set on one monitor's page only.
    auth_secret_name: Option<String>,
    auth_user: Option<String>,
    auth_header: Option<String>,
    created_at: i64,
    next_check_at: i64,
    /// `up`, `down`, `issue` or `unknown` (waiting for the first check).
    state: &'static str,
    last: Option<LastCheck>,
}

#[derive(Serialize)]
struct LastCheck {
    checked_at: i64,
    ok: bool,
    /// `200 in 182ms`, or the start of the error.
    summary: String,
}

/// What a supervisor sees of a request: header names and whether a body is sent, never the
/// values, which often carry credentials (technical-specs 07: owner and super admin only).
const HIDDEN: &str = "(hidden)";

fn hide_values(headers: &str) -> String {
    headers
        .lines()
        .map(|l| match l.split_once(':') {
            Some((name, _)) => format!("{name}: {HIDDEN}"),
            None => HIDDEN.to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn view(m: Monitor, state: &'static str, read_only: bool) -> MonitorView {
    let headers = m.headers.unwrap_or_default();
    MonitorView {
        id: m.id,
        every: interval_label(m.interval_s),
        interval_s: m.interval_s,
        timeout_s: m.timeout_ms / 1000,
        expected_status: m.expected_status,
        keyword: m.keyword,
        skip_tls_verify: m.skip_tls_verify,
        severity: m.severity,
        is_public: m.is_public,
        alert: m.alert,
        auto_incident: m.auto_incident,
        fails_to_open: m.fails_to_open,
        method: m.method,
        headers: if read_only {
            hide_values(&headers)
        } else {
            headers
        },
        body: m
            .body
            .map(|b| if read_only { HIDDEN.to_string() } else { b }),
        auth_kind: m.auth_kind,
        auth_secret_id: m.auth_secret_id,
        auth_secret_name: None,
        auth_user: m.auth_user,
        auth_header: m.auth_header,
        created_at: m.created_at,
        next_check_at: m.next_check_at,
        state,
        last: m.last.as_ref().map(|c: &CheckRow| LastCheck {
            checked_at: c.checked_at,
            ok: c.ok,
            summary: check_summary(c),
        }),
        name: m.name,
        url: m.url,
    }
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

#[derive(Serialize)]
struct DetailView {
    monitor: MonitorView,
    stats: Vec<views::StatView>,
    insight: views::InsightView,
    /// The newest checks, `PAGE` at most, before the cursor.
    checks: Vec<CheckRow>,
    /// The cursor for the next older page, when there is one.
    older: Option<String>,
}

async fn detail(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, id)): Path<(String, i64)>,
    Query(paging): Query<Paging>,
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
    let s = checks::item_state(m.last.is_some(), incident).0;
    let off = crate::platform::settings::utc_offset(&state.db.read().await?).await?;
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
    let secret_name = match m.auth_secret_id {
        Some(sid) => projects::secrets::name_of(&state.db.read().await?, p.id, sid).await?,
        None => None,
    };
    Ok(data(DetailView {
        monitor: MonitorView {
            auth_secret_name: secret_name,
            ..view(m, s, user.is_read_only())
        },
        stats,
        insight: insight.into(),
        checks: rows,
        older,
    }))
}

/// Strings, not numbers: a blank or bad number must reach `validate` and come back as that
/// field's error.
#[derive(Deserialize, Default)]
#[serde(default)]
struct MonitorForm {
    name: String,
    url: String,
    interval_s: String,
    timeout_s: String,
    expected_status: String,
    keyword: String,
    skip_tls_verify: bool,
    severity: String,
    is_public: bool,
    /// Missing means on, so a client that never sends it keeps alerts and incidents.
    alert: Option<bool>,
    auto_incident: Option<bool>,
    method: String,
    headers: String,
    body: String,
    /// Missing means 2, the count every monitor used before it could choose.
    fails_to_open: Option<String>,
    auth_kind: String,
    auth_secret_id: Option<i64>,
    auth_user: String,
    auth_header: String,
}

impl From<MonitorForm> for MonitorInput {
    fn from(f: MonitorForm) -> Self {
        let keyword = f.keyword.trim();
        MonitorInput {
            name: f.name.trim().to_string(),
            url: f.url.trim().to_string(),
            // 0 fails validation with the field's own message.
            interval_s: f.interval_s.trim().parse().unwrap_or(0),
            timeout_ms: f
                .timeout_s
                .trim()
                .parse::<i64>()
                .unwrap_or(0)
                .saturating_mul(1000),
            expected_status: f.expected_status.trim().to_string(),
            keyword: (!keyword.is_empty()).then(|| keyword.to_string()),
            skip_tls_verify: f.skip_tls_verify,
            severity: f.severity,
            is_public: f.is_public,
            alert: f.alert.unwrap_or(true),
            auto_incident: f.auto_incident.unwrap_or(true),
            fails_to_open: f.fails_to_open.map_or(2, |n| n.trim().parse().unwrap_or(0)),
            method: f.method.trim().to_uppercase(),
            headers: f.headers,
            body: Some(f.body).filter(|b| !b.trim().is_empty()),
            auth_kind: f.auth_kind.trim().to_string(),
            auth_secret_id: f.auth_secret_id,
            auth_user: Some(f.auth_user.trim().to_string()).filter(|u| !u.is_empty()),
            auth_header: Some(f.auth_header.trim().to_string()).filter(|h| !h.is_empty()),
        }
    }
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
