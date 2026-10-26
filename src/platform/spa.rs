//! The React frontend, built by `make web` into `web/dist` and embedded here. Every page path
//! that no route claims gets `index.html`; the page's router takes it from there.

use axum::{
    extract::Path,
    http::{Method, StatusCode, Uri, header},
    response::{IntoResponse, Response},
};
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "web/dist/"]
struct Web;

/// `/assets/<name>-<hash>.<ext>`: the name carries the content hash, so caching is safe for a year.
pub async fn asset(Path(path): Path<String>) -> Response {
    let Some(file) = Web::get(&format!("assets/{path}")) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    (
        [
            (header::CONTENT_TYPE, file.metadata.mimetype().to_string()),
            (
                header::CACHE_CONTROL,
                "public, max-age=31536000, immutable".to_string(),
            ),
        ],
        file.data,
    )
        .into_response()
}

/// The page shell for any GET no route claims. API and machine paths stay 404.
pub async fn fallback(method: Method, uri: Uri) -> Response {
    let path = uri.path();
    let machine = ["/api/", "/ping/", "/static/", "/assets/"]
        .iter()
        .any(|p| path.starts_with(p));
    if machine {
        return super::api_error(StatusCode::NOT_FOUND, "not_found", "No such endpoint.");
    }
    // A write to a path no route claims is 404, not 405: an unregistered route, such as
    // reset-state in prod (SEC-17), must look absent.
    if method != Method::GET && method != Method::HEAD {
        return StatusCode::NOT_FOUND.into_response();
    }
    match Web::get("index.html") {
        Some(file) => (
            [
                (header::CONTENT_TYPE, "text/html; charset=utf-8".to_string()),
                (header::CACHE_CONTROL, "no-cache".to_string()),
            ],
            file.data,
        )
            .into_response(),
        None => (
            StatusCode::SERVICE_UNAVAILABLE,
            "The web frontend is not built, so run `make web` and build Monita again.",
        )
            .into_response(),
    }
}
