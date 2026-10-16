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
