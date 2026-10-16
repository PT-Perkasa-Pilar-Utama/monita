//! One HTTP check. Never fails: every problem becomes a `CheckResult` with `ok = false`.

use std::time::{Duration, Instant};

use super::CheckResult;
use crate::platform::now;

/// Body bytes read when a keyword must be found (SEC-15).
const MAX_BODY: usize = 1024 * 1024;

/// Two clients: one verifies TLS, one does not, for monitors with self-signed internal certs.
#[derive(Clone)]
pub struct Clients {
    strict: reqwest::Client,
    insecure: reqwest::Client,
}

impl Clients {
    /// Outgoing webhooks verify TLS.
    pub fn webhook(&self) -> &reqwest::Client {
        &self.strict
    }

    pub fn new() -> anyhow::Result<Self> {
        let base = || {
            reqwest::Client::builder()
                .user_agent(concat!(
                    "Monita/",
                    env!("CARGO_PKG_VERSION"),
                    " (uptime check)"
                ))
                .redirect(reqwest::redirect::Policy::limited(5))
        };
        Ok(Self {
            strict: base().build()?,
            insecure: base().danger_accept_invalid_certs(true).build()?,
        })
    }
}

/// What a probe needs from a monitor.
pub struct Target<'a> {
    pub url: &'a str,
    pub timeout_ms: i64,
    pub expected_status: &'a str,
    pub keyword: Option<&'a str>,
    pub skip_tls_verify: bool,
}

/// `2xx` matches 200 to 299; `200,204` matches either code.
pub fn status_matches(expected: &str, code: u16) -> bool {
    let code = format!("{code:03}");
    expected.split(',').map(str::trim).any(|p| {
        p.len() == 3
            && p.bytes()
                .zip(code.bytes())
                .all(|(e, c)| e == b'x' || e == c)
    })
}

pub async fn probe(t: &Target<'_>, clients: &Clients) -> CheckResult {
    let client = if t.skip_tls_verify {
        &clients.insecure
    } else {
        &clients.strict
    };
    let checked_at = now();
    let start = Instant::now();
    let fail = |code: Option<u16>, start: Instant, error: String| CheckResult {
        checked_at,
        ok: false,
        status_code: code,
        latency_ms: start.elapsed().as_millis() as i64,
        error: Some(error),
    };
    let request = client
        .get(t.url)
        .timeout(Duration::from_millis(t.timeout_ms as u64))
        .send();
    let mut resp = match request.await {
        Ok(r) => r,
        Err(e) if e.is_timeout() => {
            return fail(None, start, format!("Timeout after {} ms", t.timeout_ms));
        }
        Err(e) if e.is_connect() => {
            return fail(
                None,
                start,
                format!("Connection failed: {}", root_cause(&e)),
            );
        }
        Err(e) => return fail(None, start, root_cause(&e)),
    };
    let code = resp.status().as_u16();
    if !status_matches(t.expected_status, code) {
        return fail(
            Some(code),
            start,
            format!("HTTP {code}, expected {}", t.expected_status),
        );
    }
    if let Some(keyword) = t.keyword.filter(|k| !k.is_empty()) {
        let mut body = Vec::new();
        loop {
            match resp.chunk().await {
                Ok(Some(chunk)) => {
                    body.extend_from_slice(&chunk);
                    if body.len() >= MAX_BODY {
                        body.truncate(MAX_BODY);
                        break;
                    }
                }
                Ok(None) => break,
                Err(e) if e.is_timeout() => {
                    return fail(
                        Some(code),
                        start,
                        format!("Timeout after {} ms", t.timeout_ms),
                    );
                }
                Err(e) => return fail(Some(code), start, root_cause(&e)),
            }
        }
        if !String::from_utf8_lossy(&body).contains(keyword) {
            return fail(
                Some(code),
                start,
                format!("Keyword \"{keyword}\" not found"),
            );
        }
    }
    CheckResult {
        checked_at,
        ok: true,
        status_code: Some(code),
        latency_ms: start.elapsed().as_millis() as i64,
        error: None,
    }
}

/// The innermost error message, which names the real problem (DNS, refused, TLS).
fn root_cause(e: &(dyn std::error::Error + 'static)) -> String {
    let mut cur = e;
    while let Some(next) = cur.source() {
        cur = next;
    }
    cur.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_classes_and_lists_match() {
        assert!(status_matches("2xx", 200));
        assert!(status_matches("2xx", 299));
        assert!(!status_matches("2xx", 301));
        assert!(status_matches("200,204", 204));
        assert!(!status_matches("200,204", 201));
        assert!(status_matches("3xx", 302));
    }
}
