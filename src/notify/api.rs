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
            "/api/ui/projects/{slug}/alerts/{route}/enabled",
            put(set_enabled),
        )
        .route(
            "/api/ui/projects/{slug}/ticket-alerts/test",
            post(test_ticket_alerts),
        )
        .merge(super::telegram::api::router())
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
    let mut routes = super::routes(&state.db, p.id).await?;
    if user.is_read_only() {
        for r in &mut routes {
            r.webhook_url = hidden(r.webhook_url.take());
            r.webhook_key = hidden(r.webhook_key.take());
        }
    }
    Ok(data(routes))
}

/// A webhook URL is its own credential: whoever holds it can post to the team's channel. A
/// supervisor sees that one is set, never the address.
fn hidden(url: Option<String>) -> Option<String> {
    url.map(|_| "(hidden)".to_string())
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
    /// `json`, `discord`, `telegram`, `ntfy` or `pagerduty`; missing is `json` (v1.1 Q17).
    #[serde(default)]
    webhook_format: String,
    /// Telegram's chat ID or PagerDuty's routing key.
    #[serde(default)]
    webhook_key: String,
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
    let form = super::RouteForm {
        emails: &f.emails,
        webhook_url: &f.webhook_url,
        use_webhook: f.use_webhook,
        webhook_format: &f.webhook_format,
        webhook_key: &f.webhook_key,
    };
    match super::save_route(&state.db, p.id, &severity, &form).await {
        Ok(()) => Ok(StatusCode::NO_CONTENT.into_response()),
        Err(RouteError::Other(e)) => Err(e.into()),
        Err(e @ (RouteError::BadWebhook | RouteError::NtfyTopic)) => {
            Err(UiError::field("webhook_url", e.to_string()))
        }
        Err(e @ (RouteError::BadFormat | RouteError::NoBot)) => {
            Err(UiError::field("webhook_format", e.to_string()))
        }
        Err(e @ RouteError::BadKey(_)) => Err(UiError::field("webhook_key", e.to_string())),
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
    let mut alerts = super::ticket_alerts(&state.db, p.id).await?;
    if user.is_read_only() {
        alerts.webhook_url = hidden(alerts.webhook_url.take());
    }
    Ok(data(alerts))
}

#[derive(Deserialize)]
struct TicketAlertsInput {
    #[serde(default)]
    digest: bool,
    #[serde(default)]
    webhook_url: String,
    #[serde(default)]
    use_webhook: bool,
    #[serde(default)]
    webhook_moves: bool,
}

/// The owner's daily digest, the webhook per new ticket and the webhook per move, each on or off.
async fn save_ticket_alerts(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
    Body(f): Body<TicketAlertsInput>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    let saved = super::save_ticket_alerts(
        &state.db,
        p.id,
        f.digest,
        &f.webhook_url,
        f.use_webhook,
        f.webhook_moves,
    );
    match saved.await {
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

#[derive(Deserialize)]
struct EnabledInput {
    enabled: bool,
}

/// Turns `critical`, `warning` or `ticket` alerts on or off; the recipients stay.
async fn set_enabled(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, route)): Path<(String, String)>,
    Body(f): Body<EnabledInput>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    if super::set_enabled(&state.db, p.id, &route, f.enabled).await? {
        Ok(StatusCode::NO_CONTENT.into_response())
    } else {
        Err(UiError::NotFound)
    }
}
