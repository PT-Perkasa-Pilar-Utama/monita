//! Monitor pages. Contract: docs/api-specs/04-checks.md section 4.3.

use askama::Template;
use axum::{
    Form, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use serde::Deserialize;
use std::collections::HashMap;

use super::{
    CheckRow, FieldErrors, INTERVALS_S, Monitor, MonitorInput,
    chart::Range,
    insight::{Insight, Stat},
    overview::{self, Day, check_summary, interval_label},
};
use crate::{
    accounts::CurrentUser,
    app::AppState,
    incidents::{self, ItemState, SourceKind},
    platform::{
        caps, now,
        page::{ListFilter, PageError, Shell},
        render::render,
        time,
    },
    projects::{self, Project},
};

const PAGE: i64 = 25;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/projects/{slug}/monitors", get(list).post(create))
        .route("/projects/{slug}/monitors/new", get(new_form))
        .route("/projects/{slug}/monitors/{id}", get(detail).post(update))
        .route("/projects/{slug}/monitors/{id}/edit", get(edit_form))
        .route("/projects/{slug}/monitors/{id}/delete", post(delete))
}

#[derive(Template)]
#[template(path = "checks/monitors.html")]
struct ListPage {
    shell: Shell,
    project: Project,
    monitors: Vec<Monitor>,
    states: HashMap<(SourceKind, i64), ItemState>,
    days: HashMap<i64, Day>,
    filter: ListFilter,
    now: i64,
}

impl ListPage {
    fn rows(&self) -> Vec<&Monitor> {
        let keep = |m: &&Monitor| {
            self.filter
                .keeps(&[&m.name, &m.url], self.state(m).0, &m.severity)
        };
        self.monitors.iter().filter(keep).collect()
    }
    fn day(&self, m: &Monitor) -> Option<&Day> {
        self.days.get(&m.id)
    }
    fn summary(&self) -> (&'static str, String) {
        let states: Vec<_> = self.monitors.iter().map(|m| self.state(m).0).collect();
        overview::summary(&states, "monitor")
    }
    fn state(&self, m: &Monitor) -> (&'static str, &'static str) {
        super::item_state(
            m.last.is_some(),
            self.states.get(&(SourceKind::Monitor, m.id)).copied(),
        )
    }
    fn every(&self, m: &Monitor) -> &'static str {
        interval_label(m.interval_s)
    }
    fn last(&self, m: &Monitor) -> String {
        m.last
            .as_ref()
            .map(check_summary)
            .unwrap_or_else(|| "no checks yet".into())
    }
    fn ago(&self, m: &Monitor) -> String {
        m.last
            .as_ref()
            .map(|c| time::ago(c.checked_at, self.now))
            .unwrap_or_default()
    }
}

async fn list(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
    Query(filter): Query<ListFilter>,
) -> Result<Response, PageError> {
    let project = projects::find_for(&state.db, &user, &slug).await?;
    let monitors = super::list_monitors(&state.db, project.id).await?;
    let states = incidents::states_for(&state.db, project.id).await?;
    let now = now();
    let days = overview::last_day(&state.db, project.id, now).await?;
    Ok(render(&ListPage {
        shell: projects::shell(&state.db, &user, "monitors", Some(&project)).await?,
        project,
        monitors,
        states,
        days,
        filter,
        now,
    }))
}

#[derive(Template)]
#[template(path = "checks/monitor_form.html")]
struct FormPage {
    shell: Shell,
    project: Project,
    /// `None` when creating.
    id: Option<i64>,
    input: MonitorInput,
    errors: FieldErrors,
}

impl FormPage {
    fn err(&self, field: &str) -> &'static str {
        self.errors.get(field).copied().unwrap_or("")
    }
    fn intervals(&self) -> Vec<(i64, &'static str)> {
        INTERVALS_S
            .iter()
            .map(|&s| (s, interval_label(s)))
            .collect()
    }
    fn timeout_s(&self) -> i64 {
        self.input.timeout_ms / 1000
    }
    fn keyword(&self) -> &str {
        self.input.keyword.as_deref().unwrap_or("")
    }
    fn action(&self) -> String {
        match self.id {
            Some(id) => format!("/projects/{}/monitors/{id}", self.project.slug),
            None => format!("/projects/{}/monitors", self.project.slug),
        }
    }
}

/// Strings, not numbers: a blank or bad number must reach `validate` and re-render the form,
/// not fail inside the extractor with a plain-text 422.
#[derive(Deserialize, Default)]
#[serde(default)]
struct MonitorForm {
    name: String,
    url: String,
    interval_s: String,
    timeout_s: String,
    expected_status: String,
    keyword: String,
    skip_tls_verify: Option<String>,
    severity: String,
    is_public: Option<String>,
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
            skip_tls_verify: f.skip_tls_verify.is_some(),
            severity: f.severity,
            is_public: f.is_public.is_some(),
        }
    }
}

async fn new_form(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
) -> Result<Response, PageError> {
    let project = projects::find_for(&state.db, &user, &slug).await?;
    Ok(render(&FormPage {
        shell: projects::shell(&state.db, &user, "monitors", Some(&project)).await?,
        project,
        id: None,
        input: MonitorInput::defaults(),
        errors: FieldErrors::new(),
    }))
}

async fn create(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
    Form(f): Form<MonitorForm>,
) -> Result<Response, PageError> {
    let project = projects::find_for(&state.db, &user, &slug).await?;
    let input = MonitorInput::from(f);
    let mut errors = input.validate();
    if errors.is_empty() {
        match super::create_monitor(&state.db, project.id, &input).await? {
            Some(id) => {
                return Ok(Redirect::to(&format!("/projects/{slug}/monitors/{id}")).into_response());
            }
            None => {
                errors.insert("name", caps::MONITORS_FULL);
            }
        }
    }
    let page = FormPage {
        shell: projects::shell(&state.db, &user, "monitors", Some(&project)).await?,
        project,
        id: None,
        input,
        errors,
    };
    Ok((StatusCode::UNPROCESSABLE_ENTITY, render(&page)).into_response())
}

async fn edit_form(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, id)): Path<(String, i64)>,
) -> Result<Response, PageError> {
    let project = projects::find_for(&state.db, &user, &slug).await?;
    let monitor = super::find_monitor(&state.db, project.id, id)
        .await?
        .ok_or(PageError::NotFound)?;
    Ok(render(&FormPage {
        shell: projects::shell(&state.db, &user, "monitors", Some(&project)).await?,
        project,
        id: Some(id),
        input: monitor.input(),
        errors: FieldErrors::new(),
    }))
}

async fn update(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, id)): Path<(String, i64)>,
    Form(f): Form<MonitorForm>,
) -> Result<Response, PageError> {
    let project = projects::find_for(&state.db, &user, &slug).await?;
    let input = MonitorInput::from(f);
    let errors = input.validate();
    if !errors.is_empty() {
        let page = FormPage {
            shell: projects::shell(&state.db, &user, "monitors", Some(&project)).await?,
            project,
            id: Some(id),
            input,
            errors,
        };
        return Ok((StatusCode::UNPROCESSABLE_ENTITY, render(&page)).into_response());
    }
    if !super::update_monitor(&state.db, project.id, id, &input).await? {
        return Err(PageError::NotFound);
    }
    Ok(Redirect::to(&format!("/projects/{slug}/monitors/{id}")).into_response())
}

async fn delete(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, id)): Path<(String, i64)>,
) -> Result<Response, PageError> {
    let project = projects::find_for(&state.db, &user, &slug).await?;
    super::delete_monitor(&state.db, &state.config.base_url, project.id, id).await?;
    Ok(Redirect::to(&format!("/projects/{slug}/monitors")).into_response())
}

#[derive(Template)]
#[template(path = "checks/monitor.html")]
struct DetailPage {
    shell: Shell,
    project: Project,
    monitor: Monitor,
    checks: Vec<CheckRow>,
    older: Option<String>,
    incident: Option<ItemState>,
    insight: Insight,
}

impl DetailPage {
    fn state(&self) -> (&'static str, &'static str) {
        super::item_state(self.monitor.last.is_some(), self.incident)
    }
    fn stats(&self) -> [Stat; 4] {
        let i = &self.insight;
        let last = match &self.monitor.last {
            Some(c) => (
                "Last check",
                time::ago(c.checked_at, i.now),
                format!("{} in {}", self.code(c), self.latency(c)),
                if c.ok { "up" } else { "down" },
            ),
            None => (
                "Last check",
                "Never".into(),
                "the first runs soon".into(),
                "plain",
            ),
        };
        let mean = ("Mean response", i.mean(), i.range_word(), "plain");
        [
            i.streak_stat(self.monitor.last.is_some()),
            last,
            mean,
            i.incidents_stat(),
        ]
    }
    fn every(&self) -> &'static str {
        interval_label(self.monitor.interval_s)
    }
    fn utc(&self, ts: &i64) -> String {
        time::utc(*ts)
    }
    fn latency(&self, c: &CheckRow) -> String {
        c.latency_ms.map(|l| format!("{l}ms")).unwrap_or_default()
    }
    fn code(&self, c: &CheckRow) -> String {
        c.status_code
            .map(|s| s.to_string())
            .unwrap_or_else(|| "none".into())
    }
}

#[derive(Deserialize)]
pub(super) struct Paging {
    /// A cursor the page itself links to; a hand-edited bad value shows the first page.
    before: Option<String>,
    /// The chart range: hour, day, week or month. Anything else is a day.
    range: Option<String>,
}

impl Paging {
    /// `<time>.<id>`: two rows can share a second, so the time alone would skip one.
    pub(super) fn cursor(&self) -> Option<(i64, i64)> {
        let (t, id) = self.before.as_deref()?.split_once('.')?;
        Some((t.parse().ok()?, id.parse().ok()?))
    }

    pub(super) fn range(&self) -> Range {
        Range::parse(self.range.as_deref())
    }
}

async fn detail(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, id)): Path<(String, i64)>,
    Query(paging): Query<Paging>,
) -> Result<Response, PageError> {
    let project = projects::find_for(&state.db, &user, &slug).await?;
    let monitor = super::find_monitor(&state.db, project.id, id)
        .await?
        .ok_or(PageError::NotFound)?;
    let checks = super::recent_checks(&state.db, id, paging.cursor(), PAGE).await?;
    let older = (checks.len() as i64 == PAGE)
        .then(|| checks.last().map(|c| format!("{}.{}", c.checked_at, c.id)))
        .flatten();
    let incident = incidents::states_for(&state.db, project.id)
        .await?
        .get(&(SourceKind::Monitor, id))
        .copied();
    let insight = Insight::monitor(&state.db, &monitor, paging.range(), now()).await?;
    Ok(render(&DetailPage {
        shell: projects::shell(&state.db, &user, "monitors", Some(&project)).await?,
        project,
        monitor,
        checks,
        older,
        incident,
        insight,
    }))
}
