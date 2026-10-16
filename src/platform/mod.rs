//! platform: configuration, database, migrations, rendering, assets, health, reset-state.
//! Spec: docs/technical-specs/05-module-definitions.md section 5.8.

pub mod assets;
pub mod config;
pub mod db;
pub mod headers;
pub mod health;
pub mod limits;
pub mod migrate;
pub mod page;
pub mod render;
pub mod reset_state;
pub mod settings;
pub mod time;
pub mod token;

use std::net::{IpAddr, SocketAddr};

use axum::{extract::ConnectInfo, http::request::Parts};

/// Current time in Unix seconds, UTC.
pub fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// The caller's IP: the socket peer, or the first `X-Forwarded-For` entry behind a trusted proxy.
// ponytail: trusts the left-most X-Forwarded-For entry; switch to right-most-untrusted if the proxy chain grows.
pub fn client_ip(parts: &Parts, trust_proxy: bool) -> Option<IpAddr> {
    if trust_proxy
        && let Some(ip) = parts
            .headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(',').next())
            .and_then(|v| v.trim().parse().ok())
    {
        return Some(ip);
    }
    parts
        .extensions
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ConnectInfo(addr)| addr.ip())
}
