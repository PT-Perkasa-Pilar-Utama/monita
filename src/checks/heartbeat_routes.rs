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

use super::{
    FieldErrors,
    heartbeat::{self, GRACES_S, Heartbeat, HeartbeatInput, PERIODS_S, PingKind, PingRow},
};
use crate::{
    accounts::CurrentUser,
    app::AppState,
    platform::{
        client_ip, now,
        page::{PageError, Shell},
        rate_limit::Verdict,
        render::render,
        settings, time,
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
    now: i64,
}

impl ListPage {
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
    Ok(render(&ListPage {
        shell: projects::routes::shell(&user, "heartbeats", Some(&project)),
        project,
        heartbeats,
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

#[derive(Deserialize)]
struct HeartbeatForm {
    name: String,
    period_s: i64,
    grace_s: i64,
    severity: String,
    is_public: Option<String>,
}

impl From<HeartbeatForm> for HeartbeatInput {
    fn from(f: HeartbeatForm) -> Self {
        HeartbeatInput {
            name: f.name.trim().to_string(),
            period_s: f.period_s,
            grace_s: f.grace_s,
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
        shell: projects::routes::shell(user, "heartbeats", Some(&project)),
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
    heartbeat::delete(&state.db, project.id, id).await?;
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
}

impl DetailPage {
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
    Ok(render(&DetailPage {
        shell: projects::routes::shell(&user, "heartbeats", Some(&project)),
        ping_url: ping_url(&state, &hb),
        project,
        hb,
        pings,
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
) -> Response {
    let found = match heartbeat::find_by_token(&state.db, token).await {
        Ok(found) => found,
        Err(e) => {
            tracing::error!(error = %e, "ping lookup failed");
            return plain(StatusCode::INTERNAL_SERVER_ERROR, "error");
        }
    };
    let Some(hb) = found else {
        return plain(StatusCode::NOT_FOUND, "not found");
    };
    // AC-13.02: a ping over the limit gets 429. The ping already counted this second keeps the heartbeat alive.
    let limit = match state.db.read().await {
        Ok(c) => settings::get(&c, "rl_ping_per_s").await.unwrap_or(1),
        Err(_) => 1,
    };
    if let Verdict::Deny(retry) =
        state
            .limiter
            .hit(&format!("ping:{}", hb.id), limit.max(1) as u32, 1, now())
    {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            [(header::RETRY_AFTER, retry.to_string())],
            "slow down",
        )
            .into_response();
    }
    let ip = client_ip(headers, peer, state.config.trust_proxy).map(|ip| ip.to_string());
    match heartbeat::record_ping(&state.db, &hb, kind, ip).await {
        Ok(()) => plain(StatusCode::OK, "OK"),
        Err(e) => {
            tracing::error!(heartbeat = hb.id, error = %e, "ping write failed");
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
    ping(&state, &token, PingKind::Ok, req.headers(), peer(&req)).await
}

async fn ping_fail(
    State(state): State<AppState>,
    Path(token): Path<String>,
    req: Request,
) -> Response {
    ping(&state, &token, PingKind::Fail, req.headers(), peer(&req)).await
}
