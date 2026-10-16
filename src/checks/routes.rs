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

use super::{CheckRow, FieldErrors, INTERVALS_S, Monitor, MonitorInput};
use crate::{
    accounts::CurrentUser,
    app::AppState,
    incidents::{self, ItemState, SourceKind},
    platform::{
        now,
        page::{PageError, Shell},
        render::render,
        time,
    },
    projects::{self, Project},
};

const PAGE: i64 = 100;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/projects/{slug}/monitors", get(list).post(create))
        .route("/projects/{slug}/monitors/new", get(new_form))
        .route("/projects/{slug}/monitors/{id}", get(detail).post(update))
        .route("/projects/{slug}/monitors/{id}/edit", get(edit_form))
        .route("/projects/{slug}/monitors/{id}/delete", post(delete))
}

fn interval_label(s: i64) -> &'static str {
    match s {
        30 => "30s",
        60 => "1m",
        180 => "3m",
        300 => "5m",
        _ => "?",
    }
}

fn check_summary(c: &CheckRow) -> String {
    let latency = c.latency_ms.map(|l| format!("{l}ms")).unwrap_or_default();
    match (c.status_code, &c.error) {
        (Some(code), _) => format!("{code} · {latency}"),
        (None, Some(e)) => e.chars().take(60).collect(),
        (None, None) => latency,
    }
}

#[derive(Template)]
#[template(path = "checks/monitors.html")]
struct ListPage {
    shell: Shell,
    project: Project,
    monitors: Vec<Monitor>,
    states: HashMap<(SourceKind, i64), ItemState>,
    now: i64,
}

impl ListPage {
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
) -> Result<Response, PageError> {
    let project = projects::find_for(&state.db, &user, &slug).await?;
    let monitors = super::list_monitors(&state.db, project.id).await?;
    let states = incidents::states_for(&state.db, project.id).await?;
    Ok(render(&ListPage {
        shell: projects::shell(&user, "monitors", Some(&project)),
        project,
        monitors,
        states,
        now: now(),
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
        shell: projects::shell(&user, "monitors", Some(&project)),
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
    let errors = input.validate();
    if !errors.is_empty() {
        let page = FormPage {
            shell: projects::shell(&user, "monitors", Some(&project)),
            project,
            id: None,
            input,
            errors,
        };
        return Ok((StatusCode::UNPROCESSABLE_ENTITY, render(&page)).into_response());
    }
    let id = super::create_monitor(&state.db, project.id, &input).await?;
    Ok(Redirect::to(&format!("/projects/{slug}/monitors/{id}")).into_response())
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
        shell: projects::shell(&user, "monitors", Some(&project)),
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
            shell: projects::shell(&user, "monitors", Some(&project)),
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
}

impl DetailPage {
    fn state(&self) -> (&'static str, &'static str) {
        super::item_state(self.monitor.last.is_some(), self.incident)
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
struct Paging {
    /// A cursor the page itself links to; a hand-edited bad value shows the first page.
    before: Option<String>,
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
    // `checked_at.id`: two checks can share a second, so the time alone would skip one.
    let before = paging.before.and_then(|b| {
        let (t, id) = b.split_once('.')?;
        Some((t.parse().ok()?, id.parse().ok()?))
    });
    let checks = super::recent_checks(&state.db, id, before, PAGE).await?;
    let older = (checks.len() as i64 == PAGE)
        .then(|| checks.last().map(|c| format!("{}.{}", c.checked_at, c.id)))
        .flatten();
    let incident = incidents::states_for(&state.db, project.id)
        .await?
        .get(&(SourceKind::Monitor, id))
        .copied();
    Ok(render(&DetailPage {
        shell: projects::shell(&user, "monitors", Some(&project)),
        project,
        monitor,
        checks,
        older,
        incident,
    }))
}
