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
    /// `http`, `tcp` or `dns`.
    kind: String,
    /// The URL, `host:port` for TCP, or the host name for DNS.
    url: String,
    /// DNS only: the address one answer must equal.
    expected_address: Option<String>,
    /// HTTPS only: days of warning before the certificate expires; null is off.
    cert_warn_days: Option<i64>,
    /// The `notAfter` of the certificate the last HTTPS check saw.
    cert_expires_at: Option<i64>,
    interval_s: i64,
    /// `30 seconds`, `1 minute`, `3 minutes` or `5 minutes`.
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

/// What a supervisor sees of a request: header names and whether a body or a sign-in name is
/// sent, never the values, which often carry credentials (technical-specs 07: owner and super admin only).
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

/// A URL's query and fragment, where tokens such as `?apikey=` live, read `(hidden)`.
fn hide_query(url: &str) -> String {
    match url.find(['?', '#']) {
        Some(i) => format!("{}{}{HIDDEN}", &url[..i], &url[i..=i]),
        None => url.to_string(),
    }
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
        auth_user: m
            .auth_user
            .map(|u| if read_only { HIDDEN.to_string() } else { u }),
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
        kind: m.kind,
        url: if read_only { hide_query(&m.url) } else { m.url },
        expected_address: m.expected_address,
        cert_warn_days: m.cert_warn_days,
        cert_expires_at: m.cert_expires_at,
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

#[cfg(test)]
mod tests {
    use super::hide_query;

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
