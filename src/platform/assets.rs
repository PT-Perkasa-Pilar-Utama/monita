//! Embedded static files: the theme script, the favicon and the fonts.

use axum::{
    extract::Path,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "static/"]
struct Static;

/// A year-long cache in a release build: `web/build.ts` puts a content hash in the theme script's
/// URL, and the favicon and the fonts never change under their names.
pub async fn serve(Path(path): Path<String>) -> Response {
    let Some(file) = Static::get(&path) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let cache = if cfg!(debug_assertions) {
        "no-cache"
    } else {
        "public, max-age=31536000, immutable"
    };
    (
        [
            (header::CONTENT_TYPE, file.metadata.mimetype().to_string()),
            (header::CACHE_CONTROL, cache.to_string()),
        ],
        file.data,
    )
        .into_response()
}
