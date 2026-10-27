//! Fixed-window rate limits in memory (SEC-12). One process, so no shared store is needed.

use std::{collections::HashMap, sync::Mutex};

#[derive(Default)]
pub struct Limiter {
    windows: Mutex<Windows>,
}

#[derive(Default)]
struct Windows {
    /// Per key: the window's start, its count, and its end, after which it can never deny again.
    by_key: HashMap<String, (i64, u32, i64)>,
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
        // Once a second at most, so the map stays bounded by the keys of windows still open.
        if now > w.swept_at {
            w.swept_at = now;
            w.by_key.retain(|_, (_, _, end)| *end > now);
        }
        let end = window_start + window_s;
        let entry = w
            .by_key
            .entry(key.to_string())
            .or_insert((window_start, 0, end));
        if entry.0 != window_start {
            *entry = (window_start, 0, end);
        }
        if entry.1 >= limit {
            return Verdict::Deny((window_start + window_s - now).max(1));
        }
        entry.1 += 1;
        Verdict::Allow
    }

    /// Takes back one hit counted for `key` in the window that contains `now`, for a request that
    /// had to count before its outcome was known. Nothing changes once that window has passed.
    pub fn refund(&self, key: &str, window_s: i64, now: i64) {
        let window_start = now - now.rem_euclid(window_s);
        let mut w = self.windows.lock().expect("rate limit lock");
        if let Some(entry) = w.by_key.get_mut(key)
            && entry.0 == window_start
        {
            entry.1 = entry.1.saturating_sub(1);
        }
    }

    /// Like `hit`, but counts nothing: is `key` already at `limit` in the current window?
    pub fn over(&self, key: &str, limit: u32, window_s: i64, now: i64) -> Verdict {
        let window_start = now - now.rem_euclid(window_s);
        let w = self.windows.lock().expect("rate limit lock");
        match w.by_key.get(key) {
            Some(&(start, n, _)) if start == window_start && n >= limit => {
                Verdict::Deny((window_start + window_s - now).max(1))
            }
            _ => Verdict::Allow,
        }
    }
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
    let message = format!("Too many requests, so retry in {retry_after} s.");
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
    fn a_refund_gives_back_one_hit_of_the_same_window_only() {
        let l = Limiter::default();
        assert!(matches!(l.hit("k", 1, 60, 120), Verdict::Allow));
        l.refund("k", 60, 130);
        assert!(
            matches!(l.hit("k", 1, 60, 140), Verdict::Allow),
            "given back"
        );
        l.refund("k", 60, 190);
        assert!(
            matches!(l.hit("k", 1, 60, 150), Verdict::Deny(_)),
            "a refund in a later window touches nothing"
        );
        l.refund("never", 60, 150);
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
    fn a_long_window_outlives_the_sweep_of_short_ones() {
        let l = Limiter::default();
        for _ in 0..3 {
            l.hit("2fa:1", 3, 900, 1);
        }
        l.hit("ip:a", 60, 60, 1);
        l.hit("ip:b", 60, 60, 500);
        assert_eq!(
            l.windows.lock().unwrap().by_key.len(),
            2,
            "ip:a's minute is over"
        );
        assert!(matches!(l.over("2fa:1", 3, 900, 500), Verdict::Deny(400)));
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
}
