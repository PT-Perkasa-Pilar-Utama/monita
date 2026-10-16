//! Renders an askama template into a response.

use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};

pub fn render<T: Template>(t: &T) -> Response {
    match t.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            tracing::error!(error = %e, "template render failed");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// Public pages may be cached for 30 s by browsers and proxies. Only a success is: a cached 500
/// would outlive its cause.
pub fn public_cache(mut res: Response) -> Response {
    if res.status().is_success() {
        res.headers_mut().insert(
            axum::http::header::CACHE_CONTROL,
            axum::http::HeaderValue::from_static("public, max-age=30"),
        );
    }
    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_success_is_publicly_cached() {
        let ok = public_cache(StatusCode::OK.into_response());
        assert_eq!(ok.headers()["cache-control"], "public, max-age=30");
        let failed = public_cache(StatusCode::INTERNAL_SERVER_ERROR.into_response());
        assert!(failed.headers().get("cache-control").is_none());
    }
}
