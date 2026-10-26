//! The React frontend, built by `make web` into `web/dist` and embedded here. Every page path
//! that no route claims gets `index.html`, or `status.html` under `/status/`; the page's router
//! takes it from there.

use axum::{
    extract::Path,
    http::{HeaderMap, HeaderValue, Method, StatusCode, Uri, header},
    response::{IntoResponse, Response},
};
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "web/dist/"]
struct Web;

/// `/assets/<name>-<hash>.<ext>`: the name carries the content hash, so caching is safe for a year.
/// The `.br` or `.gz` copy that `web/build.ts` made goes out when the browser takes it.
pub async fn asset(Path(path): Path<String>, headers: HeaderMap) -> Response {
    let name = format!("assets/{path}");
    let Some(file) = Web::get(&name) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let mime = file.metadata.mimetype().to_string();
    let packed = super::compress::precompressed(&headers)
        .find_map(|(suffix, coding)| Web::get(&format!("{name}{suffix}")).map(|f| (f, coding)));
    let (data, coding) = match packed {
        Some((f, coding)) => (f.data, Some(coding)),
        None => (file.data, None),
    };
    let mut res = (
        [
            (header::CONTENT_TYPE, mime),
            (
                header::CACHE_CONTROL,
                "public, max-age=31536000, immutable".to_string(),
            ),
            (header::VARY, "accept-encoding".to_string()),
        ],
        data,
    )
        .into_response();
    if let Some(coding) = coding {
        res.headers_mut()
            .insert(header::CONTENT_ENCODING, HeaderValue::from_static(coding));
    }
    res
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
    // A status page visitor gets the public app alone, not the team's pages (web/build.ts).
    let shell = if path.starts_with("/status/") {
        "status.html"
    } else {
        "index.html"
    };
    match Web::get(shell) {
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
