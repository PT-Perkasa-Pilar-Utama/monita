//! The project icon, an image a page links to. Contract: docs/api-specs/03-projects.md.

use axum::{
    Router,
    extract::{Path, State},
    http::header,
    response::{IntoResponse, Response},
    routing::get,
};

use crate::{accounts::CurrentUser, app::AppState, platform::page::PageError};

pub fn router() -> Router<AppState> {
    Router::new().route("/projects/{slug}/icon", get(icon))
}

/// The icon's URL carries `?v={icon_at}`, so a new icon is a new URL and the old one never changes.
async fn icon(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
) -> Result<Response, PageError> {
    let project = super::find_for(&state.db, &user, &slug).await?;
    let (mime, data) = super::icon(&state.db, project.id)
        .await?
        .ok_or(PageError::NotFound)?;
    let headers = [
        (header::CONTENT_TYPE, mime),
        (
            header::CACHE_CONTROL,
            "private, max-age=31536000, immutable".to_string(),
        ),
    ];
    Ok((headers, data).into_response())
}
