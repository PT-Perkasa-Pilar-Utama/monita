//! Fixed-window rate limits in memory (SEC-12). One process, so no shared store is needed.

use std::{collections::HashMap, sync::Mutex};

/// The longest window any caller uses. Older windows can never deny again.
const LONGEST_WINDOW_S: i64 = 60;

#[derive(Default)]
pub struct Limiter {
    windows: Mutex<Windows>,
}

#[derive(Default)]
struct Windows {
    by_key: HashMap<String, (i64, u32)>,
    swept_at: i64,
}

pub enum Verdict {
    Allow,
    /// Seconds until the window resets.
    Deny(i64),
}

impl Limiter {
    /// Counts one hit for `key` in the window of `window_s` seconds that contains `now`.
    pub fn hit(&self, key: &str, limit: u32, window_s: i64, now: i64) -> Verdict {
        let window_start = now - now.rem_euclid(window_s);
        let mut w = self.windows.lock().expect("rate limit lock");
        // Once a second at most, so the map stays bounded by the keys seen in the last minute.
        if now > w.swept_at {
            w.swept_at = now;
            w.by_key
                .retain(|_, (start, _)| *start >= now - LONGEST_WINDOW_S);
        }
        let entry = w.by_key.entry(key.to_string()).or_insert((window_start, 0));
        if entry.0 != window_start {
            *entry = (window_start, 0);
        }
        if entry.1 >= limit {
            return Verdict::Deny((window_start + window_s - now).max(1));
        }
        entry.1 += 1;
        Verdict::Allow
    }

    /// Like `hit`, but counts nothing: is `key` already at `limit` in the current window?
    pub fn over(&self, key: &str, limit: u32, window_s: i64, now: i64) -> Verdict {
        let window_start = now - now.rem_euclid(window_s);
        let w = self.windows.lock().expect("rate limit lock");
        match w.by_key.get(key) {
            Some(&(start, n)) if start == window_start && n >= limit => {
                Verdict::Deny((window_start + window_s - now).max(1))
            }
            _ => Verdict::Allow,
        }
    }
}

/// The bucket name for a caller's IP. IPv6 is keyed on its /64: one host owns the whole prefix
/// and could otherwise rotate addresses past every per-IP limit.
pub fn ip_key(ip: Option<std::net::IpAddr>) -> String {
    match ip {
        Some(std::net::IpAddr::V6(v6)) => {
            let prefix = u128::from(v6) & (u128::MAX << 64);
            format!("{}/64", std::net::Ipv6Addr::from(prefix))
        }
        Some(v4) => v4.to_string(),
        None => "unknown".into(),
    }
}

/// Paths limited per client IP (SEC-12): public pages, sign-in, public and ticket APIs.
/// `/ping/` has its own limit; `/healthz`, assets and signed-in pages are not limited here.
// ponytail: a hand-kept prefix list, which a new public route can miss; move to a layer on the
// public sub-routers if the public surface grows past these prefixes.
pub fn ip_limited(method: &axum::http::Method, path: &str) -> bool {
    path.starts_with("/status/")
        || path.starts_with("/api/ui/public/")
        || path.starts_with("/api/v1/public/")
        || path.starts_with("/api/v1/tickets")
        || (method == axum::http::Method::POST
            && (path == "/api/ui/login" || path.starts_with("/api/ui/invites/")))
}

/// 429 with `Retry-After`: the JSON envelope under `/api/`, plain text elsewhere. The routes that
/// browsers call (api-specs 01 section 1.8) also get `Access-Control-Allow-Origin: *`, so a page
/// can read the 429; `/api/v1/ref-ids` is for backends and gets none.
pub fn too_many(path: &str, retry_after: i64) -> axum::response::Response {
    use axum::{
        http::{StatusCode, header},
        response::IntoResponse,
    };
    let retry = [(header::RETRY_AFTER, retry_after.to_string())];
    let message = format!("Too many requests. Retry in {retry_after} s.");
    if path.starts_with("/api/") {
        let mut res = super::api_error(StatusCode::TOO_MANY_REQUESTS, "rate_limited", &message);
        if path.starts_with("/api/v1/public/") || path.starts_with("/api/v1/tickets") {
            res.headers_mut().insert(
                header::ACCESS_CONTROL_ALLOW_ORIGIN,
                axum::http::HeaderValue::from_static("*"),
            );
        }
        (retry, res).into_response()
    } else {
        (StatusCode::TOO_MANY_REQUESTS, retry, message).into_response()
    }
}

/// Middleware for SEC-12's per-IP limit, read from `settings.rl_ip_per_min` on each request.
pub async fn per_ip(
    axum::extract::State(state): axum::extract::State<crate::app::AppState>,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    if req.method() == axum::http::Method::OPTIONS || !ip_limited(req.method(), req.uri().path()) {
        return next.run(req).await;
    }
    let ip = ip_key(super::caller_ip(
        req.headers(),
        req.extensions(),
        state.config.trust_proxy,
    ));
    // ponytail: one settings read per limited request; cache the value in AppState, refreshed by
    // the scheduler tick, if public traffic makes this read show up.
    let limit = match state.db.read().await {
        Ok(c) => super::settings::get_or(&c, "rl_ip_per_min", 60).await,
        Err(e) => {
            tracing::warn!(error = %e, "no read connection; using the default IP limit");
            60
        }
    };
    if let Verdict::Deny(retry) = state.limiter.hit(
        &format!("ip:{ip}"),
        u32::try_from(limit.max(1)).unwrap_or(u32::MAX),
        60,
        super::now(),
    ) {
        return too_many(req.uri().path(), retry);
    }
    next.run(req).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_up_to_the_limit_per_window_then_resets() {
        let l = Limiter::default();
        assert!(matches!(l.hit("k", 2, 60, 120), Verdict::Allow));
        assert!(matches!(l.hit("k", 2, 60, 130), Verdict::Allow));
        assert!(matches!(l.hit("k", 2, 60, 150), Verdict::Deny(30)));
        assert!(matches!(l.hit("other", 2, 60, 150), Verdict::Allow));
        assert!(
            matches!(l.hit("k", 2, 60, 180), Verdict::Allow),
            "new window"
        );
    }

    #[test]
    fn expired_windows_are_dropped_so_the_map_stays_bounded() {
        let l = Limiter::default();
        for ip in 0..1_000 {
            l.hit(&format!("ip:{ip}"), 60, 60, 120);
        }
        l.hit("ip:new", 60, 60, 181);
        assert_eq!(l.windows.lock().unwrap().by_key.len(), 1);
    }

    #[test]
    fn only_browser_routes_get_cors_on_a_429() {
        let public = too_many("/api/v1/public/status/x", 5);
        assert_eq!(public.headers()["access-control-allow-origin"], "*");
        let backend = too_many("/api/v1/ref-ids", 5);
        assert!(
            backend
                .headers()
                .get("access-control-allow-origin")
                .is_none()
        );
        assert_eq!(backend.headers()["retry-after"], "5");
    }

    #[test]
    fn ipv6_callers_share_a_bucket_per_64() {
        let a = ip_key(Some("2001:db8:1:2::1".parse().unwrap()));
        let b = ip_key(Some("2001:db8:1:2:ffff::9".parse().unwrap()));
        let other = ip_key(Some("2001:db8:1:3::1".parse().unwrap()));
        assert_eq!(a, b);
        assert_ne!(a, other);
        assert_eq!(ip_key(Some("10.0.0.1".parse().unwrap())), "10.0.0.1");
    }

    #[test]
    fn over_reports_a_full_window_without_counting() {
        let l = Limiter::default();
        assert!(matches!(l.over("k", 1, 60, 120), Verdict::Allow));
        l.hit("k", 1, 60, 120);
        assert!(matches!(l.over("k", 1, 60, 130), Verdict::Deny(50)));
        assert!(
            matches!(l.over("k", 1, 60, 180), Verdict::Allow),
            "new window"
        );
    }

    #[test]
    fn public_pages_and_their_json_are_limited_per_ip_but_signed_in_json_is_not() {
        use axum::http::Method;
        for path in [
            "/status/shop/report",
            "/api/ui/public/shop/report",
            "/api/ui/public/shop/tickets",
        ] {
            assert!(ip_limited(&Method::POST, path), "{path}");
        }
        assert!(!ip_limited(&Method::GET, "/api/ui/projects/shop/tickets"));
        assert!(!ip_limited(&Method::GET, "/api/ui/login"));
    }
}
