//! A project's alerts as JSON: the preview and ticket alerts. Alert cards have their own
//! routes (`cards::api`). Contract: docs/api-specs/06-notify.md.

use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, put},
};
use serde::{Deserialize, Serialize};

use super::AlertError;
use crate::{
    accounts::CurrentUser,
    app::AppState,
    platform::ui::{Body, UiError, UiResult, data},
    projects,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/ui/projects/{slug}/alerts-preview", get(preview))
        .route(
            "/api/ui/projects/{slug}/ticket-alerts",
            get(ticket_alerts).put(save_ticket_alerts),
        )
        .route(
            "/api/ui/projects/{slug}/ticket-alerts/enabled",
            put(set_ticket_enabled),
        )
        .route(
            "/api/ui/projects/{slug}/ticket-alerts/test",
            post(test_ticket_alerts),
        )
        .merge(super::cards::api::router())
        .merge(super::telegram::api::router())
}

/// A webhook URL is its own credential: whoever holds it can post to the team's channel. A
/// supervisor sees that one is set, never the address.
fn hidden(url: Option<String>) -> Option<String> {
    url.map(|_| "(hidden)".to_string())
}

#[derive(Serialize)]
struct Tested {
    /// How many recipients the test went to; 0 when there are none.
    sent_to: usize,
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
        alerts.webhook_key = hidden(alerts.webhook_key.take());
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
    #[serde(default)]
    webhook_format: String,
    #[serde(default)]
    webhook_key: String,
}

/// The owner's daily digest, the webhook per new ticket and the webhook per move, each on or off.
async fn save_ticket_alerts(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
    Body(f): Body<TicketAlertsInput>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    let form = super::TicketForm {
        digest: f.digest,
        webhook_url: &f.webhook_url,
        use_webhook: f.use_webhook,
        webhook_moves: f.webhook_moves,
        webhook_format: &f.webhook_format,
        webhook_key: &f.webhook_key,
    };
    match super::save_ticket_alerts(&state.db, p.id, &form).await {
        Ok(()) => Ok(StatusCode::NO_CONTENT.into_response()),
        Err(AlertError::Other(e)) => Err(e.into()),
        Err(e @ (AlertError::BadFormat | AlertError::NoBot)) => {
            Err(UiError::field("webhook_format", e.to_string()))
        }
        Err(e @ AlertError::BadKey(_)) => Err(UiError::field("webhook_key", e.to_string())),
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

/// Ticket alerts on or off; their settings stay.
async fn set_ticket_enabled(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
    Body(f): Body<EnabledInput>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    super::set_ticket_enabled(&state.db, p.id, f.enabled).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
