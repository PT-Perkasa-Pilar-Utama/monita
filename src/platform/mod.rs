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
    if trust_proxy
        && let Some(ip) = headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.rsplit(',').next())
            .and_then(|v| v.trim().parse().ok())
    {
        return Some(ip);
    }
    peer.map(|addr| addr.ip())
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
    }
}
