//! Embedded static files: CSS, htmx, icons, fonts.

use axum::{
    extract::Path,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "static/"]
struct Static;

/// Appended as `?v=` to asset URLs, so a release busts the browser cache.
// ponytail: package version, not a content hash; swap to rust-embed's sha256 if assets change between releases.
pub const ASSET_VERSION: &str = env!("CARGO_PKG_VERSION");

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
