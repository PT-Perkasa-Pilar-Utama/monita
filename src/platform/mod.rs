//! platform: configuration, database, migrations, rendering, assets, health, reset-state.
//! Spec: docs/technical-specs/05-module-definitions.md section 5.8.

pub mod assets;
pub mod backup;
pub mod config;
pub mod db;
pub mod headers;
pub mod health;
pub mod jobs;
pub mod limits;
pub mod migrate;
pub mod page;
pub mod rate_limit;
pub mod render;
pub mod reset_state;
pub mod settings;
pub mod time;
pub mod token;

use std::net::{IpAddr, SocketAddr};

use axum::http::HeaderMap;

/// The JSON error envelope of api-specs 01: `{"error": {"code", "message"}}`.
pub fn api_error(
    status: axum::http::StatusCode,
    code: &str,
    message: &str,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    (
        status,
        axum::Json(serde_json::json!({ "error": { "code": code, "message": message } })),
    )
        .into_response()
}

/// An `http` or `https` URL with a host, as the HTTP client will accept it. Monitor URLs,
/// webhook URLs and ticket page URLs all go through it. The raw text is stored, so it must be
/// what the parser read, up to scheme and host case: no whitespace or control characters, which the parser drops silently,
/// and the `http://` prefix written out, which it repairs.
pub fn valid_http_url(raw: &str) -> bool {
    raw.len() <= 2000
        && ["http://", "https://"].iter().any(|p| {
            raw.get(..p.len())
                .is_some_and(|s| s.eq_ignore_ascii_case(p))
        })
        && !raw.chars().any(|c| c.is_whitespace() || c.is_control())
        && reqwest::Url::parse(raw).is_ok_and(|u| {
            matches!(u.scheme(), "http" | "https") && u.host_str().is_some_and(|h| !h.is_empty())
        })
}

/// Current time in Unix seconds, UTC.
pub fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// The caller's IP: the socket peer, or the right-most `X-Forwarded-For` entry behind a trusted
/// proxy. That entry is the one our proxy appended; everything left of it is client-supplied.
// ponytail: one trusted proxy; with a chain, skip that many entries from the right.
pub fn client_ip(
    headers: &HeaderMap,
    peer: Option<SocketAddr>,
    trust_proxy: bool,
) -> Option<IpAddr> {
    // The last header line: a proxy may add its own line instead of appending to the client's.
    if trust_proxy
        && let Some(ip) = headers
            .get_all("x-forwarded-for")
            .iter()
            .next_back()
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.rsplit(',').next())
            .and_then(|v| v.trim().parse::<IpAddr>().ok())
    {
        return Some(ip.to_canonical());
    }
    // A dual-stack listener sees IPv4 callers as `::ffff:a.b.c.d`; allowlists name them as IPv4.
    peer.map(|addr| addr.ip().to_canonical())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn behind_a_proxy_the_right_most_forwarded_entry_wins() {
        let mut h = HeaderMap::new();
        h.insert(
            "x-forwarded-for",
            "10.0.0.1, 10.0.0.2, 203.0.113.9".parse().unwrap(),
        );
        let peer: SocketAddr = "127.0.0.1:5000".parse().unwrap();
        assert_eq!(
            client_ip(&h, Some(peer), true),
            Some("203.0.113.9".parse().unwrap())
        );
        assert_eq!(
            client_ip(&h, Some(peer), false),
            Some(peer.ip()),
            "headers are ignored without a trusted proxy"
        );
        h.append("x-forwarded-for", "198.51.100.7".parse().unwrap());
        assert_eq!(
            client_ip(&h, Some(peer), true),
            Some("198.51.100.7".parse().unwrap()),
            "the proxy's own line wins"
        );
    }

    #[test]
    fn a_url_needs_an_http_scheme_and_a_host() {
        assert!(valid_http_url("https://shop.example/cart"));
        assert!(valid_http_url("http://10.0.0.5:8080/health"));
        assert!(
            valid_http_url("HTTPS://Shop.example/cart"),
            "schemes are case-insensitive"
        );
        for bad in [
            "http://",
            "https://:80",
            "ftp://x.co",
            "javascript:alert(1)",
            "x.co",
            "http:x.co",
            "https://x.co/a\tb",
            "https://x.co/\n",
            " https://x.co",
        ] {
            assert!(!valid_http_url(bad), "{bad}");
        }
    }

    #[test]
    fn an_ipv4_caller_on_a_dual_stack_socket_reads_as_ipv4() {
        let peer: SocketAddr = "[::ffff:10.20.1.5]:5000".parse().unwrap();
        assert_eq!(
            client_ip(&HeaderMap::new(), Some(peer), false),
            Some("10.20.1.5".parse().unwrap())
        );
    }
}
