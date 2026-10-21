//! One check: HTTP, a TCP connection or a DNS lookup (v1.1 Q16). Never fails: every problem
//! becomes a `CheckResult` with `ok = false`.

pub mod cert;
pub mod icmp;

use std::time::{Duration, Instant};

use super::CheckResult;
use crate::platform::now;

/// Body bytes read when a keyword must be found (SEC-15).
const MAX_BODY: usize = 1024 * 1024;

/// One client per (TLS check, redirect policy). Skipping TLS serves self-signed internal certs.
/// A monitor that expects a 3xx sees the redirect itself instead of where it leads.
#[derive(Clone)]
pub struct Clients {
    strict: reqwest::Client,
    insecure: reqwest::Client,
    strict_no_follow: reqwest::Client,
    insecure_no_follow: reqwest::Client,
}

impl Clients {
    /// Outgoing webhooks verify TLS and never follow a redirect.
    pub fn webhook(&self) -> &reqwest::Client {
        &self.strict_no_follow
    }

    pub fn new() -> anyhow::Result<Self> {
        let base = |follow: bool| {
            reqwest::Client::builder()
                .user_agent(concat!(
                    "Monita/",
                    env!("CARGO_PKG_VERSION"),
                    " (uptime check)"
                ))
                .redirect(if follow {
                    reqwest::redirect::Policy::limited(5)
                } else {
                    reqwest::redirect::Policy::none()
                })
                // The leaf certificate, for its expiry (v1.1 Q25).
                .tls_info(true)
        };
        Ok(Self {
            strict: base(true).build()?,
            insecure: base(true).danger_accept_invalid_certs(true).build()?,
            strict_no_follow: base(false).build()?,
            insecure_no_follow: base(false).danger_accept_invalid_certs(true).build()?,
        })
    }
}

pub struct Target<'a> {
    pub url: &'a str,
    pub method: &'a str,
    /// Parsed from the monitor's `Name: value` lines.
    pub headers: Vec<(String, String)>,
    /// Sent with POST and PUT only.
    pub body: Option<&'a str>,
    pub timeout_ms: i64,
    pub expected_status: &'a str,
    pub keyword: Option<&'a str>,
    pub skip_tls_verify: bool,
    pub auth: Option<Auth<'a>>,
}

/// How a check signs in, with the secret's value.
pub enum Auth<'a> {
    Bearer(&'a str),
    Basic { user: &'a str, password: &'a str },
    Header { name: &'a str, value: &'a str },
}

/// The methods a monitor may send. Only POST and PUT carry a body.
pub const METHODS: [&str; 4] = ["GET", "HEAD", "POST", "PUT"];

/// The most headers one monitor sends, and the longest value.
const MAX_HEADERS: usize = 20;
const MAX_HEADER_VALUE: usize = 4096;

/// `Name: value` lines, as curl shows them. A blank line is skipped; a value keeps its inner
/// colons. `Host` and `Content-Length` are refused: the client sets them.
pub fn parse_headers(text: &str) -> Result<Vec<(String, String)>, &'static str> {
    let mut out = Vec::new();
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let Some((name, value)) = line.split_once(':') else {
            return Err("Write one header per line, as Name: value.");
        };
        let (name, value) = (name.trim(), value.trim());
        if reqwest::header::HeaderName::from_bytes(name.as_bytes()).is_err()
            || reqwest::header::HeaderValue::from_str(value).is_err()
        {
            return Err("Write one header per line, as Name: value.");
        }
        if name.eq_ignore_ascii_case("host") || name.eq_ignore_ascii_case("content-length") {
            return Err("Monita sets Host and Content-Length itself; remove them.");
        }
        if value.len() > MAX_HEADER_VALUE {
            return Err("A header value can be at most 4 KB.");
        }
        out.push((name.to_string(), value.to_string()));
    }
    if out.len() > MAX_HEADERS {
        return Err("Send at most 20 headers.");
    }
    Ok(out)
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
    let follow = !t
        .expected_status
        .split(',')
        .any(|p| p.trim().starts_with('3'));
    let client = match (t.skip_tls_verify, follow) {
        (false, true) => &clients.strict,
        (true, true) => &clients.insecure,
        (false, false) => &clients.strict_no_follow,
        (true, false) => &clients.insecure_no_follow,
    };
    let checked_at = now();
    let start = Instant::now();
    let fail = |code: Option<u16>, start: Instant, error: String| CheckResult {
        checked_at,
        ok: false,
        status_code: code,
        latency_ms: start.elapsed().as_millis() as i64,
        error: Some(error),
        cert_expires_at: None,
    };
    let method = reqwest::Method::from_bytes(t.method.as_bytes()).unwrap_or(reqwest::Method::GET);
    let mut request = client
        .request(method.clone(), t.url)
        .timeout(Duration::from_millis(t.timeout_ms as u64));
    for (name, value) in &t.headers {
        request = request.header(name.as_str(), value.as_str());
    }
    request = match t.auth {
        None => request,
        Some(Auth::Bearer(token)) => request.bearer_auth(token),
        Some(Auth::Basic { user, password }) => request.basic_auth(user, Some(password)),
        Some(Auth::Header { name, value }) => request.header(name, value),
    };
    if let Some(body) = t
        .body
        .filter(|_| method == reqwest::Method::POST || method == reqwest::Method::PUT)
    {
        request = request.body(body.to_string());
    }
    let request = request.send();
    let resp = match request.await {
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
    let cert = resp
        .extensions()
        .get::<reqwest::tls::TlsInfo>()
        .and_then(reqwest::tls::TlsInfo::peer_certificate)
        .and_then(cert::not_after);
    CheckResult {
        cert_expires_at: cert,
        ..judge(t, resp, checked_at, start).await
    }
}

/// Status and keyword of a response that arrived.
async fn judge(
    t: &Target<'_>,
    mut resp: reqwest::Response,
    checked_at: i64,
    start: Instant,
) -> CheckResult {
    let fail = |code: Option<u16>, start: Instant, error: String| CheckResult {
        error: Some(error),
        status_code: code,
        ..failed(checked_at, start, String::new())
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
        ok: true,
        status_code: Some(code),
        error: None,
        ..failed(checked_at, start, String::new())
    }
}

/// A failed result started at `checked_at`, `start` ago.
fn failed(checked_at: i64, start: Instant, error: String) -> CheckResult {
    CheckResult {
        checked_at,
        ok: false,
        status_code: None,
        latency_ms: start.elapsed().as_millis() as i64,
        error: Some(error),
        cert_expires_at: None,
    }
}

/// A passed result started at `checked_at`, `start` ago.
fn passed(checked_at: i64, start: Instant) -> CheckResult {
    CheckResult {
        ok: true,
        error: None,
        ..failed(checked_at, start, String::new())
    }
}

/// Passes when a TCP connection to `host:port` opens within the timeout. The name is resolved
/// first, inside the same timeout.
pub async fn probe_tcp(target: &str, timeout_ms: i64) -> CheckResult {
    let checked_at = now();
    let start = Instant::now();
    let limit = Duration::from_millis(timeout_ms as u64);
    match tokio::time::timeout(limit, tokio::net::TcpStream::connect(target)).await {
        Ok(Ok(_)) => passed(checked_at, start),
        Ok(Err(e)) => failed(checked_at, start, format!("Connection failed: {e}")),
        Err(_) => failed(checked_at, start, format!("Timeout after {timeout_ms} ms")),
    }
}

/// Passes when `host` resolves with the server's resolver and, when `expected` is set, one A or
/// AAAA answer equals it. A failure names the answers it got.
pub async fn probe_dns(host: &str, expected: Option<&str>, timeout_ms: i64) -> CheckResult {
    let checked_at = now();
    let start = Instant::now();
    let limit = Duration::from_millis(timeout_ms as u64);
    let answers: Vec<std::net::IpAddr> =
        match tokio::time::timeout(limit, tokio::net::lookup_host((host, 0))).await {
            Ok(Ok(addrs)) => addrs.map(|a| a.ip()).collect(),
            Ok(Err(e)) => return failed(checked_at, start, format!("Lookup failed: {e}")),
            Err(_) => return failed(checked_at, start, format!("Timeout after {timeout_ms} ms")),
        };
    let wanted = expected.and_then(|e| e.parse::<std::net::IpAddr>().ok());
    match wanted {
        _ if answers.is_empty() => failed(checked_at, start, "No address in the answer".into()),
        Some(ip) if !answers.contains(&ip) => {
            let got: Vec<String> = answers.iter().take(5).map(ToString::to_string).collect();
            failed(
                checked_at,
                start,
                format!("Resolved to {}, expected {ip}", got.join(", ")),
            )
        }
        _ => passed(checked_at, start),
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

    #[test]
    fn header_lines_parse_as_curl_shows_them() {
        let h = parse_headers("Authorization: Bearer a:b\n\n  X-Env :  prod  \n").unwrap();
        assert_eq!(
            h,
            [
                ("Authorization".to_string(), "Bearer a:b".to_string()),
                ("X-Env".to_string(), "prod".to_string())
            ]
        );
        assert!(parse_headers("").unwrap().is_empty());
        assert!(parse_headers("no colon here").is_err());
        assert!(parse_headers("Host: example.com").is_err());
        assert!(parse_headers("Bad Name: x").is_err());
        assert!(parse_headers(&"A: 1\n".repeat(21)).is_err());
    }
}
