//! `POST .../monitors/batch` and `.../heartbeats/batch`: pause, resume or delete many at once.

use axum::{
    Router,
    extract::{Path, State},
    routing::post,
};
use serde::Deserialize;

use crate::{
    accounts::CurrentUser,
    app::AppState,
    checks::batch::{self, Action, BatchError},
    incidents::SourceKind,
    platform::ui::{Body, UiError, UiResult, data},
    projects,
};

pub(super) fn project_routes() -> Router<AppState> {
    Router::new()
        .route("/monitors/batch", post(monitors))
        .route("/heartbeats/batch", post(heartbeats))
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct BatchForm {
    /// `pause`, `resume` or `delete`.
    action: String,
    ids: Vec<i64>,
}

async fn monitors(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
    Body(f): Body<BatchForm>,
) -> UiResult {
    run(&state, &user, &slug, SourceKind::Monitor, f).await
}

async fn heartbeats(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
    Body(f): Body<BatchForm>,
) -> UiResult {
    run(&state, &user, &slug, SourceKind::Heartbeat, f).await
}

/// 200 `{count}` when every id was done; nothing changes when one is refused.
async fn run(
    state: &AppState,
    user: &crate::accounts::User,
    slug: &str,
    kind: SourceKind,
    f: BatchForm,
) -> UiResult {
    let p = projects::find_for(&state.db, user, slug).await?;
    let action = Action::parse(f.action.trim())
        .ok_or_else(|| UiError::field("action", "Pick pause, resume or delete."))?;
    match batch::apply(
        &state.db,
        &state.config.base_url,
        p.id,
        kind,
        &f.ids,
        action,
    )
    .await
    {
        Ok(count) => Ok(data(serde_json::json!({ "count": count }))),
        Err(BatchError::Unknown) => Err(UiError::NotFound),
        Err(BatchError::Other(e)) => Err(e.into()),
        Err(e) => Err(UiError::field("ids", e.to_string())),
    }
}
