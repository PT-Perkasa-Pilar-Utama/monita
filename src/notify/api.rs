//! A project's alert preview as JSON. Alert cards (`cards::api`) and Telegram (`telegram::api`)
//! have their own routes. Contract: docs/api-specs/06-notify.md.

use axum::{
    Router,
    extract::{Path, State},
    routing::get,
};

use crate::{
    accounts::CurrentUser,
    app::AppState,
    platform::ui::{UiResult, data},
    projects,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/ui/projects/{slug}/alerts-preview", get(preview))
        .merge(super::cards::api::router())
        .merge(super::telegram::api::router())
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
