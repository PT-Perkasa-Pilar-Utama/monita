//! A project's alert cards as JSON. Contract: docs/api-specs/06-notify.md section 6.1.

use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, put},
};
use serde::Deserialize;
use serde_json::json;

use super::{AlertError, CardForm};
use crate::{
    accounts::CurrentUser,
    app::AppState,
    platform::ui::{Body, UiError, UiResult, created, data},
    projects,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/ui/projects/{slug}/alert-cards",
            get(list).post(create),
        )
        .route(
            "/api/ui/projects/{slug}/alert-cards/{id}",
            put(update).delete(remove),
        )
        .route(
            "/api/ui/projects/{slug}/alert-cards/{id}/enabled",
            put(set_enabled),
        )
        .route("/api/ui/projects/{slug}/alert-cards/{id}/test", post(test))
}

impl From<AlertError> for UiError {
    fn from(e: AlertError) -> Self {
        let field = match &e {
            AlertError::Other(_) => None,
            AlertError::NotFound => return UiError::NotFound,
            AlertError::Fixed | AlertError::Full => return UiError::invalid(e.to_string()),
            AlertError::BadFormat | AlertError::NoBot | AlertError::TicketPagerDuty => Some("kind"),
            AlertError::NoName => Some("name"),
            AlertError::NoLevel => Some("levels"),
            AlertError::NoEvent => Some("events"),
            AlertError::BadEmail(_) | AlertError::TooManyEmails | AlertError::NoEmails => {
                Some("emails")
            }
            AlertError::BadWebhook | AlertError::NtfyTopic => Some("url"),
            AlertError::BadKey(_) => Some("key"),
        };
        match (field, e) {
            (_, AlertError::Other(e)) => UiError::Internal(e),
            (Some(f), e) => UiError::field(f, e.to_string()),
            (None, e) => UiError::invalid(e.to_string()),
        }
    }
}

/// A webhook URL or key is its own credential. A supervisor sees that one is set, never it.
fn hidden(v: Option<String>) -> Option<String> {
    v.map(|_| "(hidden)".to_string())
}

async fn list(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    let mut cards = super::list(&state.db, p.id).await?;
    if user.is_read_only() {
        for c in &mut cards {
            c.url = hidden(c.url.take());
            c.key = hidden(c.key.take());
        }
    }
    Ok(data(cards))
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct CardInput {
    /// `incidents` (the default) or `tickets`.
    section: String,
    kind: String,
    name: String,
    critical: bool,
    warning: bool,
    new_tickets: bool,
    ticket_moves: bool,
    /// One address a line.
    emails: String,
    url: String,
    key: String,
}

impl CardInput {
    fn form(&self) -> CardForm<'_> {
        CardForm {
            section: &self.section,
            kind: &self.kind,
            name: &self.name,
            critical: self.critical,
            warning: self.warning,
            new_tickets: self.new_tickets,
            ticket_moves: self.ticket_moves,
            emails: &self.emails,
            url: &self.url,
            key: &self.key,
        }
    }
}

/// 201 `{id}`.
async fn create(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
    Body(f): Body<CardInput>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    let id = super::create(&state.db, p.id, &f.form()).await?;
    Ok(created(json!({ "id": id })))
}

async fn update(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, id)): Path<(String, i64)>,
    Body(f): Body<CardInput>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    super::update(&state.db, p.id, id, &f.form()).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn remove(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, id)): Path<(String, i64)>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    super::delete(&state.db, p.id, id).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

#[derive(Deserialize)]
struct EnabledInput {
    enabled: bool,
}

async fn set_enabled(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, id)): Path<(String, i64)>,
    Body(f): Body<EnabledInput>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    super::set_enabled(&state.db, p.id, id, f.enabled).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

/// `{sent_to}`: how many recipients the test went to.
async fn test(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, id)): Path<(String, i64)>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    let sent_to = super::send_test(&state.db, p.id, id, &state.config.base_url).await?;
    Ok(data(json!({ "sent_to": sent_to })))
}
