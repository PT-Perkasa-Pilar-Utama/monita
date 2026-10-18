//! A project's alert routes as JSON. Contract: docs/api-specs/06-notify.md.

use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, put},
};
use serde::{Deserialize, Serialize};

use super::RouteError;
use crate::{
    accounts::CurrentUser,
    app::AppState,
    platform::ui::{Body, UiError, UiResult, data},
    projects,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/ui/projects/{slug}/alerts", get(list))
        .route("/api/ui/projects/{slug}/alerts/{severity}", put(save))
        .route("/api/ui/projects/{slug}/alerts/{severity}/test", post(test))
        .route("/api/ui/projects/{slug}/alerts-preview", get(preview))
        .route(
            "/api/ui/projects/{slug}/ticket-alerts",
            get(ticket_alerts).put(save_ticket_alerts),
        )
        .route(
            "/api/ui/projects/{slug}/ticket-alerts/test",
            post(test_ticket_alerts),
        )
}

fn severity_ok(s: &str) -> Result<(), UiError> {
    if matches!(s, "critical" | "warning") {
        Ok(())
    } else {
        Err(UiError::NotFound)
    }
}

async fn list(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    Ok(data(super::routes(&state.db, p.id).await?))
}

#[derive(Deserialize)]
struct RouteInput {
    /// One address per line.
    #[serde(default)]
    emails: String,
    #[serde(default)]
    webhook_url: String,
    #[serde(default)]
    use_webhook: bool,
}

/// US-15: who gets this severity's alerts.
async fn save(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, severity)): Path<(String, String)>,
    Body(f): Body<RouteInput>,
) -> UiResult {
    severity_ok(&severity)?;
    let p = projects::find_for(&state.db, &user, &slug).await?;
    let saved = super::save_route(
        &state.db,
        p.id,
        &severity,
        &f.emails,
        &f.webhook_url,
        f.use_webhook,
    )
    .await;
    match saved {
        Ok(()) => Ok(StatusCode::NO_CONTENT.into_response()),
        Err(RouteError::Other(e)) => Err(e.into()),
        Err(e @ RouteError::BadWebhook) => Err(UiError::field("webhook_url", e.to_string())),
        Err(e) => Err(UiError::field("emails", e.to_string())),
    }
}

#[derive(Serialize)]
struct Tested {
    /// How many recipients the test went to; 0 when the route has none.
    sent_to: usize,
}

/// US-15: queue a test alert to this severity's recipients.
async fn test(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, severity)): Path<(String, String)>,
) -> UiResult {
    severity_ok(&severity)?;
    let p = projects::find_for(&state.db, &user, &slug).await?;
    let sent_to = super::send_test(&state.db, p.id, &severity, &state.config.base_url).await?;
    Ok(data(Tested { sent_to }))
}

/// Sample alerts of every kind, as this project's recipients get them.
async fn preview(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    Ok(data(super::previews(
        &p.name,
        &p.slug,
        &state.config.base_url,
        crate::platform::now(),
    )))
}

async fn ticket_alerts(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    Ok(data(super::ticket_alerts(&state.db, p.id).await?))
}

#[derive(Deserialize)]
struct TicketAlertsInput {
    #[serde(default)]
    digest: bool,
    #[serde(default)]
    webhook_url: String,
    #[serde(default)]
    use_webhook: bool,
}

/// The owner's daily digest and the webhook per new ticket, each on or off.
async fn save_ticket_alerts(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
    Body(f): Body<TicketAlertsInput>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    match super::save_ticket_alerts(&state.db, p.id, f.digest, &f.webhook_url, f.use_webhook).await
    {
        Ok(()) => Ok(StatusCode::NO_CONTENT.into_response()),
        Err(RouteError::Other(e)) => Err(e.into()),
        Err(e) => Err(UiError::field("webhook_url", e.to_string())),
    }
}

async fn test_ticket_alerts(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    let sent_to = super::send_ticket_test(&state.db, p.id, &state.config.base_url).await?;
    Ok(data(Tested { sent_to }))
}
