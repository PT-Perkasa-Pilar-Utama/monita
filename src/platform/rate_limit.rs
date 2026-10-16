//! Fixed-window rate limits in memory (SEC-12). One process, so no shared store is needed.

use std::{collections::HashMap, sync::Mutex};

/// Windows older than this are dropped when the map grows past `SWEEP_AT` keys.
const SWEEP_AT: usize = 10_000;

#[derive(Default)]
pub struct Limiter {
    windows: Mutex<HashMap<String, (i64, u32)>>,
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
        let mut map = self.windows.lock().expect("rate limit lock");
        if map.len() > SWEEP_AT {
            map.retain(|_, (start, _)| *start >= window_start - 3600);
        }
        let entry = map.entry(key.to_string()).or_insert((window_start, 0));
        if entry.0 != window_start {
            *entry = (window_start, 0);
        }
        if entry.1 >= limit {
            return Verdict::Deny((window_start + window_s - now).max(1));
        }
        entry.1 += 1;
        Verdict::Allow
    }
}

/// Paths limited per client IP (SEC-12): public pages, sign-in, public and ticket APIs.
/// `/ping/` has its own limit; `/healthz`, assets and signed-in pages are not limited here.
pub fn ip_limited(method: &axum::http::Method, path: &str) -> bool {
    path.starts_with("/status/")
        || path.starts_with("/api/v1/public/")
        || path.starts_with("/api/v1/tickets")
        || (method == axum::http::Method::POST
            && (path == "/login" || path.starts_with("/invite/")))
}

/// 429 with `Retry-After`: the JSON envelope under `/api/`, plain text elsewhere. Under `/api/`
/// it carries `Access-Control-Allow-Origin: *`, so a browser caller can read the 429.
pub fn too_many(path: &str, retry_after: i64) -> axum::response::Response {
    use axum::{
        http::{StatusCode, header},
        response::IntoResponse,
    };
    let retry = [(header::RETRY_AFTER, retry_after.to_string())];
    let message = format!("Too many requests. Retry in {retry_after} s.");
    if path.starts_with("/api/") {
        let res = super::api_error(StatusCode::TOO_MANY_REQUESTS, "rate_limited", &message);
        (retry, [(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")], res).into_response()
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
    let peer = req
        .extensions()
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
        .map(|c| c.0);
    let ip = super::client_ip(req.headers(), peer, state.config.trust_proxy)
        .map_or_else(|| "unknown".to_string(), |ip| ip.to_string());
    let limit = match state.db.read().await {
        Ok(c) => super::settings::get(&c, "rl_ip_per_min")
            .await
            .unwrap_or(60),
        Err(_) => 60,
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
}
