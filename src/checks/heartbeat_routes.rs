//! Heartbeat pages and the ping endpoints. Contract: docs/api-specs/04-checks.md.

use std::net::SocketAddr;

use askama::Template;
use axum::{
    Form, Router,
    extract::{ConnectInfo, Path, Request, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use serde::Deserialize;
use std::collections::HashMap;

use super::{
    FieldErrors,
    heartbeat::{
        self, GRACES_S, Heartbeat, HeartbeatInput, PERIODS_S, PingKind, PingOutcome, PingRow,
    },
};
use crate::{
    accounts::CurrentUser,
    app::AppState,
    incidents::{self, ItemState, SourceKind},
    platform::{
        client_ip, now,
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
        .route("/ping/{token}", get(ping_ok).head(ping_ok).post(ping_ok))
        .route(
            "/ping/{token}/fail",
            get(ping_fail).head(ping_fail).post(ping_fail),
        )
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
    let errors = input.validate();
    if !errors.is_empty() {
        return Ok((
            StatusCode::UNPROCESSABLE_ENTITY,
            render(&form_page(&user, project, None, input, errors)),
        )
            .into_response());
    }
    let id = heartbeat::create(&state.db, project.id, &input).await?;
    Ok(Redirect::to(&format!("/projects/{slug}/heartbeats/{id}")).into_response())
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
    incident: Option<ItemState>,
}

impl DetailPage {
    fn state(&self) -> (&'static str, &'static str) {
        self.hb.state(self.incident)
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
) -> Result<Response, PageError> {
    let project = projects::find_for(&state.db, &user, &slug).await?;
    let hb = heartbeat::find(&state.db, project.id, id)
        .await?
        .ok_or(PageError::NotFound)?;
    let pings = heartbeat::recent_pings(&state.db, id, 100).await?;
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
        incident,
    }))
}

fn plain(status: StatusCode, body: &'static str) -> Response {
    (
        status,
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        body,
    )
        .into_response()
}

async fn ping(
    state: &AppState,
    token: &str,
    kind: PingKind,
    headers: &HeaderMap,
    peer: Option<SocketAddr>,
    cause: Option<String>,
) -> Response {
    let ping = heartbeat::Ping {
        token,
        kind,
        source_ip: client_ip(headers, peer, state.config.trust_proxy),
        cause,
    };
    match heartbeat::accept_ping(&state.db, &state.limiter, &state.config.base_url, ping).await {
        Ok(PingOutcome::Accepted) => plain(StatusCode::OK, "OK"),
        Ok(PingOutcome::UnknownToken) => plain(StatusCode::NOT_FOUND, "not found"),
        Ok(PingOutcome::Limited(retry)) => (
            StatusCode::TOO_MANY_REQUESTS,
            [(header::RETRY_AFTER, retry.to_string())],
            "slow down",
        )
            .into_response(),
        Err(e) => {
            tracing::error!(error = %e, "ping failed");
            plain(StatusCode::INTERNAL_SERVER_ERROR, "error")
        }
    }
}

fn peer(req: &Request) -> Option<SocketAddr> {
    req.extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|c| c.0)
}

async fn ping_ok(
    State(state): State<AppState>,
    Path(token): Path<String>,
    req: Request,
) -> Response {
    ping(
        &state,
        &token,
        PingKind::Ok,
        req.headers(),
        peer(&req),
        None,
    )
    .await
}

async fn ping_fail(
    State(state): State<AppState>,
    Path(token): Path<String>,
    req: Request,
) -> Response {
    let (headers, peer) = (req.headers().clone(), peer(&req));
    // The body, if any, is the job's own error text, at most 1 KB (api-specs 04 section 4.2).
    // A client that disconnects mid-body also lands here; it never reads the answer.
    let Ok(body) = axum::body::to_bytes(req.into_body(), 1024).await else {
        return plain(StatusCode::PAYLOAD_TOO_LARGE, "body over 1 KB");
    };
    let cause = String::from_utf8_lossy(&body).trim().to_string();
    ping(
        &state,
        &token,
        PingKind::Fail,
        &headers,
        peer,
        (!cause.is_empty()).then_some(cause),
    )
    .await
}
