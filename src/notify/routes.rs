//! Alert routes page. Contract: docs/api-specs/06-notify.md.

use askama::Template;
use axum::{
    Form, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use serde::Deserialize;

use super::{Route, RouteError};
use crate::{
    accounts::CurrentUser,
    app::AppState,
    platform::page::{PageError, Shell},
    platform::render::render,
    projects::{self, Project},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/projects/{slug}/alerts", get(page))
        .route("/projects/{slug}/alerts/{severity}", post(save))
        .route("/projects/{slug}/alerts/{severity}/test", post(test))
}

#[derive(Template)]
#[template(path = "notify/alerts.html")]
struct AlertsPage {
    shell: Shell,
    project: Project,
    routes: Vec<Route>,
    /// The severity block that carries `error` or `notice`.
    focus: String,
    error: String,
    notice: String,
}

impl AlertsPage {
    fn emails(&self, r: &Route) -> String {
        r.emails.join("\n")
    }
    fn webhook(&self, r: &Route) -> String {
        r.webhook_url.clone().unwrap_or_default()
    }
}

async fn view(
    state: &AppState,
    user: &crate::accounts::User,
    project: Project,
) -> Result<AlertsPage, PageError> {
    Ok(AlertsPage {
        shell: projects::routes::shell(user, "alerts", Some(&project)),
        routes: super::routes(&state.db, project.id).await?,
        project,
        focus: String::new(),
        error: String::new(),
        notice: String::new(),
    })
}

async fn page(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
) -> Result<Response, PageError> {
    let project = projects::find_for(&state.db, &user, &slug).await?;
    Ok(render(&view(&state, &user, project).await?))
}

#[derive(Deserialize)]
struct RouteForm {
    #[serde(default)]
    emails: String,
    #[serde(default)]
    webhook_url: String,
    use_webhook: Option<String>,
}

fn severity_ok(s: &str) -> Result<(), PageError> {
    if matches!(s, "critical" | "warning") {
        Ok(())
    } else {
        Err(PageError::NotFound)
    }
}

async fn save(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, severity)): Path<(String, String)>,
    Form(f): Form<RouteForm>,
) -> Result<Response, PageError> {
    severity_ok(&severity)?;
    let project = projects::find_for(&state.db, &user, &slug).await?;
    match super::save_route(
        &state.db,
        project.id,
        &severity,
        &f.emails,
        &f.webhook_url,
        f.use_webhook.is_some(),
    )
    .await
    {
        Ok(()) => Ok(Redirect::to(&format!("/projects/{slug}/alerts")).into_response()),
        Err(RouteError::Other(e)) => Err(e.into()),
        Err(e) => {
            let mut page = view(&state, &user, project).await?;
            if let Some(r) = page.routes.iter_mut().find(|r| r.severity == severity) {
                r.emails = f.emails.lines().map(str::to_owned).collect();
                r.webhook_url = Some(f.webhook_url);
                r.use_webhook = f.use_webhook.is_some();
            }
            page.focus = severity;
            page.error = e.to_string();
            Ok((StatusCode::UNPROCESSABLE_ENTITY, render(&page)).into_response())
        }
    }
}

async fn test(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, severity)): Path<(String, String)>,
) -> Result<Response, PageError> {
    severity_ok(&severity)?;
    let project = projects::find_for(&state.db, &user, &slug).await?;
    let n = super::send_test(&state.db, project.id, &severity, &state.config.base_url).await?;
    let mut page = view(&state, &user, project).await?;
    page.focus = severity;
    page.notice = match n {
        0 => "No recipients set for this severity.".into(),
        1 => "Test queued for 1 recipient. It arrives within a minute.".into(),
        n => format!("Test queued for {n} recipients. They arrive within a minute."),
    };
    Ok(render(&page))
}
