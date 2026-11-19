//! The project icon, an image a page links to: the team's route and the status page's.
//! Contract: docs/api-specs/03-projects.md.

use axum::{
    Router,
    extract::{Path, State},
    http::header,
    response::{IntoResponse, Response},
    routing::get,
};

use crate::{accounts::CurrentUser, app::AppState, platform::page::PageError};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/projects/{slug}/icon", get(icon))
        .route("/api/ui/public/{slug}/icon", get(public_icon))
}

/// The icon's URL carries `?v={icon_at}`, so a new icon is a new URL and the old one never changes.
async fn icon(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
) -> Result<Response, PageError> {
    let project = super::find_for(&state.db, &user, &slug).await?;
    image(&state, project.id, "private").await
}

/// The status page's header shows the icon. Any slug that opens a status page opens its icon.
async fn public_icon(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Response, PageError> {
    let project = super::find_public(&state.db, &slug)
        .await?
        .ok_or(PageError::NotFound)?;
    image(&state, project.id, "public").await
}

async fn image(state: &AppState, project_id: i64, scope: &str) -> Result<Response, PageError> {
    let (mime, data) = super::icon(&state.db, project_id)
        .await?
        .ok_or(PageError::NotFound)?;
    let headers = [
        (header::CONTENT_TYPE, mime),
        (
            header::CACHE_CONTROL,
            format!("{scope}, max-age=31536000, immutable"),
        ),
    ];
    Ok((headers, data).into_response())
}
