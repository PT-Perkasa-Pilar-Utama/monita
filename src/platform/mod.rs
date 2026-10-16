//! platform: configuration, database, migrations, rendering, assets, health, reset-state.
//! Spec: docs/technical-specs/05-module-definitions.md section 5.8.

pub mod assets;
pub mod config;
pub mod db;
pub mod headers;
pub mod health;
pub mod migrate;
pub mod render;
pub mod reset_state;

/// Current time in Unix seconds, UTC.
pub fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
