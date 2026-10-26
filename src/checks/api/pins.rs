//! Pin and unpin a monitor or heartbeat for the signed-in user (Q48). These routes are not in
//! `project_routes`, so an API key never reaches them; a supervisor does, because a pin is a
//! personal setting.

use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::put,
};

use crate::{
    accounts::{AccountUser, User},
    app::AppState,
    checks::{
        self,
        pins::{self, PinError},
    },
    incidents::SourceKind,
    platform::ui::{UiError, UiResult},
    projects,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/ui/projects/{slug}/monitors/{id}/pin",
            put(pin_monitor).delete(unpin_monitor),
        )
        .route(
            "/api/ui/projects/{slug}/heartbeats/{id}/pin",
            put(pin_heartbeat).delete(unpin_heartbeat),
        )
}

async fn pin_monitor(
    State(state): State<AppState>,
    AccountUser(user): AccountUser,
    Path((slug, id)): Path<(String, i64)>,
) -> UiResult {
    set(&state, &user, &slug, SourceKind::Monitor, id, true).await
}

async fn unpin_monitor(
    State(state): State<AppState>,
    AccountUser(user): AccountUser,
    Path((slug, id)): Path<(String, i64)>,
) -> UiResult {
    set(&state, &user, &slug, SourceKind::Monitor, id, false).await
}

async fn pin_heartbeat(
    State(state): State<AppState>,
    AccountUser(user): AccountUser,
    Path((slug, id)): Path<(String, i64)>,
) -> UiResult {
    set(&state, &user, &slug, SourceKind::Heartbeat, id, true).await
}

async fn unpin_heartbeat(
    State(state): State<AppState>,
    AccountUser(user): AccountUser,
    Path((slug, id)): Path<(String, i64)>,
) -> UiResult {
    set(&state, &user, &slug, SourceKind::Heartbeat, id, false).await
}

/// 204, 404 for a project the user cannot see or a source of another project, or 422 with the
/// cap's message once 6 of the kind are pinned in the project.
async fn set(
    state: &AppState,
    user: &User,
    slug: &str,
    kind: SourceKind,
    id: i64,
    on: bool,
) -> UiResult {
    let p = projects::find_for(&state.db, user, slug).await?;
    let name = match kind {
        SourceKind::Monitor => checks::find_monitor(&state.db, p.id, id)
            .await?
            .map(|m| m.name),
        SourceKind::Heartbeat => checks::heartbeat::find(&state.db, p.id, id)
            .await?
            .map(|h| h.name),
    };
    let Some(name) = name else {
        return Err(UiError::NotFound);
    };
    match pins::set(&state.db, user.id, p.id, kind, id, &name, on).await {
        Ok(()) => Ok(StatusCode::NO_CONTENT.into_response()),
        Err(e @ PinError::Full { .. }) => Err(UiError::invalid(e.to_string())),
        Err(PinError::Other(e)) => Err(UiError::Internal(e)),
    }
}
