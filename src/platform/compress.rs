//! Compressed answers. JSON goes out as gzip, which every browser takes, over plain HTTP too.
//! The web bundle ships precompressed as `.br` and `.gz` (`web/build.ts`), picked by `asset`.

use axum::{
    body::Body,
    extract::Request,
    http::{HeaderMap, HeaderValue, Method, header},
    middleware::Next,
    response::Response,
};

/// Smaller bodies gain less than the header costs.
const MIN_BYTES: usize = 1024;
/// gzip's default level: 259 KB of chart JSON to about 34 KB. Level 1 leaves 45 KB.
const LEVEL: u32 = 6;

/// True when `Accept-Encoding` lists `coding` without `q=0`.
pub fn accepts(headers: &HeaderMap, coding: &str) -> bool {
    headers
        .get_all(header::ACCEPT_ENCODING)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(','))
        .any(|part| {
            let mut it = part.split(';');
            let name = it.next().unwrap_or("").trim();
            let refused = it.any(|p| {
                p.trim()
                    .strip_prefix("q=")
                    .and_then(|q| q.trim().parse::<f32>().ok())
                    == Some(0.0)
            });
            name.eq_ignore_ascii_case(coding) && !refused
        })
}

/// BREACH needs a request another site starts that still carries the session cookie (a
/// top-level navigation, under `SameSite=Lax`). A browser that says where a request comes from
/// (`Sec-Fetch-Site`) gets compressed JSON only for its own pages. One that does not say (Safari
/// before 16.4, Chrome before 76) gets it only without a cookie: a script with a key, or a public
/// page.
fn same_origin(headers: &HeaderMap) -> bool {
    match headers.get("sec-fetch-site") {
        Some(v) => v == "same-origin" || v == "none",
        None => !headers.contains_key(header::COOKIE),
    }
}

/// Compresses a JSON answer with gzip when the client asks for it.
pub async fn json(req: Request, next: Next) -> Response {
    let wants = req.method() != Method::HEAD
        && accepts(req.headers(), "gzip")
        && same_origin(req.headers());
    let mut res = next.run(req).await;
    let json = res
        .headers()
        .get(header::CONTENT_TYPE)
        .is_some_and(|v| v.as_bytes().starts_with(b"application/json"));
    if !json || res.headers().contains_key(header::CONTENT_ENCODING) {
        return res;
    }
    res.headers_mut()
        .append(header::VARY, HeaderValue::from_static("accept-encoding"));
    if !wants {
        return res;
    }
    let (mut parts, body) = res.into_parts();
    let bytes = match axum::body::to_bytes(body, usize::MAX).await {
        Ok(b) => b,
        Err(e) => {
            tracing::error!(error = %e, "reading a JSON answer to compress it");
            return super::api_error(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "internal",
                "Monita could not send the answer.",
            );
        }
    };
    if bytes.len() < MIN_BYTES {
        return Response::from_parts(parts, Body::from(bytes));
    }
    match gzip(&bytes) {
        Ok(z) => {
            parts.headers.remove(header::CONTENT_LENGTH);
            parts
                .headers
                .insert(header::CONTENT_ENCODING, HeaderValue::from_static("gzip"));
            Response::from_parts(parts, Body::from(z))
        }
        Err(e) => {
            tracing::error!(error = %e, "gzip failed, so the answer goes out plain");
            Response::from_parts(parts, Body::from(bytes))
        }
    }
}

fn gzip(bytes: &[u8]) -> std::io::Result<Vec<u8>> {
    use std::io::Write;
    let mut z = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::new(LEVEL));
    z.write_all(bytes)?;
    z.finish()
}

/// The precompressed copy of a web file to send: its file suffix and its `Content-Encoding`.
pub fn precompressed(headers: &HeaderMap) -> impl Iterator<Item = (&'static str, &'static str)> {
    [(".br", "br"), (".gz", "gzip")]
        .into_iter()
        .filter(move |(_, coding)| accepts(headers, coding))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(pairs: &[(&'static str, &'static str)]) -> HeaderMap {
        let mut m = HeaderMap::new();
        for (k, v) in pairs {
            m.append(*k, HeaderValue::from_static(v));
        }
        m
    }

    #[test]
    fn accept_encoding_is_read_with_its_q_values() {
        let a = h(&[("accept-encoding", "gzip, deflate, br;q=0.9, zstd")]);
        assert!(accepts(&a, "zstd") && accepts(&a, "br") && accepts(&a, "gzip"));
        assert!(!accepts(&a, "compress"));
        let refused = h(&[("accept-encoding", "zstd;q=0, gzip")]);
        assert!(!accepts(&refused, "zstd"));
        assert!(!accepts(&HeaderMap::new(), "gzip"));
    }

    #[test]
    fn only_same_origin_or_cookieless_requests_get_compressed_json() {
        assert!(same_origin(&HeaderMap::new()));
        assert!(
            !same_origin(&h(&[("cookie", "monita_session=x")])),
            "an old browser with a session"
        );
        assert!(same_origin(&h(&[("sec-fetch-site", "same-origin")])));
        assert!(!same_origin(&h(&[("sec-fetch-site", "cross-site")])));
        assert!(!same_origin(&h(&[("sec-fetch-site", "same-site")])));
    }
}
