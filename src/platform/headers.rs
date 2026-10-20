//! Security headers (SEC-05) and the Origin check against CSRF (SEC-03).

use axum::{
    extract::{Request, State},
    http::{HeaderValue, Method, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::app::AppState;

/// `style-src` allows inline styles: Radix places popovers and dialogs with a `style` attribute.
/// Scripts stay limited to Monita's own files.
const CSP: &str = "default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; script-src 'self'; font-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'self'";

pub async fn security_headers(req: Request, next: Next) -> Response {
    // Signed-in JSON can carry a one-time secret (a new API key, an invite link): no cache keeps it.
    let private = req.uri().path().starts_with("/api/ui/")
        && !req.uri().path().starts_with("/api/ui/public/");
    let mut res = next.run(req).await;
    let h = res.headers_mut();
    if private {
        h.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    }
    h.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(CSP),
    );
    h.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    h.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("same-origin"),
    );
    res
}

/// Browser writes must come from Monita's own origin: forms and the frontend's `/api/ui/` calls,
/// which carry the session cookie. Machine endpoints (`/api/v1/`, `/ping/`, reset-state)
/// authenticate with keys or tokens instead.
pub async fn origin_check(State(state): State<AppState>, req: Request, next: Next) -> Response {
    let safe = matches!(*req.method(), Method::GET | Method::HEAD | Method::OPTIONS);
    let path = req.uri().path();
    let machine =
        path.starts_with("/api/v1/") || path.starts_with("/ping/") || path == "/admin/reset-state";
    if !safe && !machine {
        let origin = req
            .headers()
            .get(header::ORIGIN)
            .and_then(|v| v.to_str().ok());
        if !origin_ok(origin, &state.config.base_url) {
            let message = "Request blocked: wrong origin.";
            if path.starts_with("/api/") {
                return super::api_error(StatusCode::FORBIDDEN, "forbidden", message);
            }
            return (StatusCode::FORBIDDEN, message).into_response();
        }
    }
    next.run(req).await
}

/// The request's `Origin` is Monita's own. When Monita's address is a loopback one, such as the
/// default `http://localhost:7466`, any loopback name for the same scheme and port counts too: a
/// page on `127.0.0.1:7466` is the same server on the same machine. Every other site stays out.
fn origin_ok(origin: Option<&str>, base_url: &str) -> bool {
    let Some(origin) = origin else {
        return false;
    };
    if origin == base_url {
        return true;
    }
    let loopback = |u: &reqwest::Url| match u.host_str() {
        Some("localhost") => true,
        Some(h) => h
            .trim_start_matches('[')
            .trim_end_matches(']')
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback()),
        None => false,
    };
    match (reqwest::Url::parse(origin), reqwest::Url::parse(base_url)) {
        (Ok(o), Ok(b)) => {
            loopback(&o)
                && loopback(&b)
                && o.scheme() == b.scheme()
                && o.port_or_known_default() == b.port_or_known_default()
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::origin_ok;

    #[test]
    fn a_loopback_address_accepts_its_other_loopback_names_and_nothing_else() {
        let base = "http://localhost:7466";
        assert!(origin_ok(Some(base), base));
        assert!(origin_ok(Some("http://127.0.0.1:7466"), base));
        assert!(origin_ok(Some("http://[::1]:7466"), base));
        assert!(
            !origin_ok(Some("http://127.0.0.1:8080"), base),
            "another port"
        );
        assert!(
            !origin_ok(Some("https://127.0.0.1:7466"), base),
            "another scheme"
        );
        assert!(!origin_ok(Some("http://evil.example"), base));
        assert!(!origin_ok(None, base));
        let public = "https://monita.example.com";
        assert!(origin_ok(Some(public), public));
        assert!(
            !origin_ok(Some("https://127.0.0.1"), public),
            "only a loopback address relaxes"
        );
    }
}
