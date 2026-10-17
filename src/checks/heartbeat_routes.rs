//! Heartbeat pages. The ping endpoints are in `ping_routes.rs`. Contract: api-specs 04.

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
    FieldErrors,
    heartbeat::{self, GRACES_S, Heartbeat, HeartbeatInput, PERIODS_S, PingRow},
    insight::{Insight, Stat},
};
use crate::{
    accounts::CurrentUser,
    app::AppState,
    incidents::{self, ItemState, SourceKind},
    platform::{
        caps, now,
        page::{PageError, Shell},
        render::render,
        time,
    },
    projects::{self, Project},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/projects/{slug}/heartbeats", get(list).post(create))
        .route("/projects/{slug}/heartbeats/new", get(new_form))
        .route("/projects/{slug}/heartbeats/{id}", get(detail).post(update))
        .route("/projects/{slug}/heartbeats/{id}/edit", get(edit_form))
        .route("/projects/{slug}/heartbeats/{id}/delete", post(delete))
}

fn ping_url(state: &AppState, hb: &Heartbeat) -> String {
    format!("{}/ping/{}", state.config.base_url, hb.token)
}

#[derive(Template)]
#[template(path = "checks/heartbeats.html")]
struct ListPage {
    shell: Shell,
    project: Project,
    heartbeats: Vec<Heartbeat>,
    states: HashMap<(SourceKind, i64), ItemState>,
    now: i64,
}

impl ListPage {
    fn state(&self, h: &Heartbeat) -> (&'static str, &'static str) {
        h.state(self.states.get(&(SourceKind::Heartbeat, h.id)).copied())
    }
    fn every(&self, h: &Heartbeat) -> String {
        heartbeat::label(&PERIODS_S, h.period_s)
    }
    fn last(&self, h: &Heartbeat) -> String {
        h.last_ping_at
            .map(|t| time::ago(t, self.now))
            .unwrap_or_else(|| "never".into())
    }
}

async fn list(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
) -> Result<Response, PageError> {
    let project = projects::find_for(&state.db, &user, &slug).await?;
    let heartbeats = heartbeat::list(&state.db, project.id).await?;
    let states = incidents::states_for(&state.db, project.id).await?;
    Ok(render(&ListPage {
        shell: projects::shell(&user, "heartbeats", Some(&project)),
        project,
        heartbeats,
        states,
        now: now(),
    }))
}

#[derive(Template)]
#[template(path = "checks/heartbeat_form.html")]
struct FormPage {
    shell: Shell,
    project: Project,
    id: Option<i64>,
    input: HeartbeatInput,
    errors: FieldErrors,
}

impl FormPage {
    fn err(&self, field: &str) -> &'static str {
        self.errors.get(field).copied().unwrap_or("")
    }
    fn periods(&self) -> &'static [(i64, &'static str)] {
        &PERIODS_S
    }
    fn graces(&self) -> &'static [(i64, &'static str)] {
        &GRACES_S
    }
    fn action(&self) -> String {
        match self.id {
            Some(id) => format!("/projects/{}/heartbeats/{id}", self.project.slug),
            None => format!("/projects/{}/heartbeats", self.project.slug),
        }
    }
}

/// Strings, not numbers, as in `routes::MonitorForm`.
#[derive(Deserialize, Default)]
#[serde(default)]
struct HeartbeatForm {
    name: String,
    period_s: String,
    grace_s: String,
    severity: String,
    is_public: Option<String>,
}

impl From<HeartbeatForm> for HeartbeatInput {
    fn from(f: HeartbeatForm) -> Self {
        HeartbeatInput {
            name: f.name.trim().to_string(),
            period_s: f.period_s.trim().parse().unwrap_or(0),
            grace_s: f.grace_s.trim().parse().unwrap_or(-1),
            severity: f.severity,
            is_public: f.is_public.is_some(),
        }
    }
}

fn form_page(
    user: &crate::accounts::User,
    project: Project,
    id: Option<i64>,
    input: HeartbeatInput,
    errors: FieldErrors,
) -> FormPage {
    FormPage {
        shell: projects::shell(user, "heartbeats", Some(&project)),
        project,
        id,
        input,
        errors,
    }
}

async fn new_form(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
) -> Result<Response, PageError> {
    let project = projects::find_for(&state.db, &user, &slug).await?;
    Ok(render(&form_page(
        &user,
        project,
        None,
        HeartbeatInput::default(),
        FieldErrors::new(),
    )))
}

async fn create(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
    Form(f): Form<HeartbeatForm>,
) -> Result<Response, PageError> {
    let project = projects::find_for(&state.db, &user, &slug).await?;
    let input = HeartbeatInput::from(f);
    let mut errors = input.validate();
    if errors.is_empty() {
        match heartbeat::create(&state.db, project.id, &input).await? {
            Some(id) => {
                return Ok(
                    Redirect::to(&format!("/projects/{slug}/heartbeats/{id}")).into_response()
                );
            }
            None => {
                errors.insert("name", caps::HEARTBEATS_FULL);
            }
        }
    }
    Ok((
        StatusCode::UNPROCESSABLE_ENTITY,
        render(&form_page(&user, project, None, input, errors)),
    )
        .into_response())
}

async fn edit_form(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, id)): Path<(String, i64)>,
) -> Result<Response, PageError> {
    let project = projects::find_for(&state.db, &user, &slug).await?;
    let hb = heartbeat::find(&state.db, project.id, id)
        .await?
        .ok_or(PageError::NotFound)?;
    Ok(render(&form_page(
        &user,
        project,
        Some(id),
        hb.input(),
        FieldErrors::new(),
    )))
}

async fn update(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, id)): Path<(String, i64)>,
    Form(f): Form<HeartbeatForm>,
) -> Result<Response, PageError> {
    let project = projects::find_for(&state.db, &user, &slug).await?;
    let input = HeartbeatInput::from(f);
    let errors = input.validate();
    if !errors.is_empty() {
        return Ok((
            StatusCode::UNPROCESSABLE_ENTITY,
            render(&form_page(&user, project, Some(id), input, errors)),
        )
            .into_response());
    }
    if !heartbeat::update(&state.db, project.id, id, &input).await? {
        return Err(PageError::NotFound);
    }
    Ok(Redirect::to(&format!("/projects/{slug}/heartbeats/{id}")).into_response())
}

async fn delete(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, id)): Path<(String, i64)>,
) -> Result<Response, PageError> {
    let project = projects::find_for(&state.db, &user, &slug).await?;
    heartbeat::delete(&state.db, &state.config.base_url, project.id, id).await?;
    Ok(Redirect::to(&format!("/projects/{slug}/heartbeats")).into_response())
}

#[derive(Template)]
#[template(path = "checks/heartbeat.html")]
struct DetailPage {
    shell: Shell,
    project: Project,
    hb: Heartbeat,
    ping_url: String,
    pings: Vec<PingRow>,
    older: Option<String>,
    incident: Option<ItemState>,
    insight: Insight,
}

impl DetailPage {
    fn state(&self) -> (&'static str, &'static str) {
        self.hb.state(self.incident)
    }
    fn stats(&self) -> [Stat; 4] {
        let i = &self.insight;
        let seen = self.hb.last_ping_at.is_some();
        let last = match self.hb.last_ping_at {
            Some(t) => (
                "Last ping",
                time::ago(t, i.now),
                format!("expected every {}", self.period()),
                "plain",
            ),
            None => (
                "Last ping",
                "Never".into(),
                format!("expected every {}", self.period()),
                "plain",
            ),
        };
        let failed = i.failed();
        let pings = (
            "Pings",
            i.series.total.to_string(),
            format!("{failed} failed, {}", i.range_word()),
            if failed > 0 { "down" } else { "plain" },
        );
        [i.streak_stat(seen), last, pings, i.incidents_stat()]
    }
    fn period(&self) -> String {
        heartbeat::label(&PERIODS_S, self.hb.period_s)
    }
    fn grace(&self) -> String {
        heartbeat::label(&GRACES_S, self.hb.grace_s)
    }
    fn utc(&self, ts: &i64) -> String {
        time::utc(*ts)
    }
}

async fn detail(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, id)): Path<(String, i64)>,
    Query(paging): Query<super::routes::Paging>,
) -> Result<Response, PageError> {
    let project = projects::find_for(&state.db, &user, &slug).await?;
    let hb = heartbeat::find(&state.db, project.id, id)
        .await?
        .ok_or(PageError::NotFound)?;
    let pings = heartbeat::recent_pings(&state.db, id, paging.cursor()).await?;
    let insight = Insight::heartbeat(&state.db, &hb, paging.range(), now()).await?;
    let older = (pings.len() as i64 == caps::PINGS_PAGE)
        .then(|| pings.last().map(|p| format!("{}.{}", p.received_at, p.id)))
        .flatten();
    let incident = incidents::states_for(&state.db, project.id)
        .await?
        .get(&(SourceKind::Heartbeat, id))
        .copied();
    Ok(render(&DetailPage {
        shell: projects::shell(&user, "heartbeats", Some(&project)),
        ping_url: ping_url(&state, &hb),
        project,
        hb,
        pings,
        older,
        incident,
        insight,
    }))
}
