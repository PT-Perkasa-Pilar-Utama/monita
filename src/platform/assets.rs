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

/// `/static/<path>?v=<content hash>`: a changed file gets a new URL, so the year-long cache is safe.
/// Fonts carry no version: only `app.css` names them, and its own URL changes with it.
pub fn url(path: &str) -> String {
    let hash: String = Static::get(path)
        .map(|f| {
            f.metadata.sha256_hash()[..4]
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect()
        })
        .unwrap_or_default();
    format!("/static/{path}?v={hash}")
}

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

#[cfg(test)]
mod tests {
    #[test]
    fn the_url_carries_the_file_content_hash() {
        let css = super::url("app.css");
        let js = super::url("app.js");
        assert!(
            css.starts_with("/static/app.css?v=") && css.len() == "/static/app.css?v=".len() + 8
        );
        assert_ne!(
            css[css.len() - 8..],
            js[js.len() - 8..],
            "different files, different hashes"
        );
    }
}
