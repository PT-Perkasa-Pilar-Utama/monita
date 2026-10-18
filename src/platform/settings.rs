//! Runtime limits in the `settings` table. Keys: technical-specs 06 section 6.3.14.

use turso::Connection;

pub const KEYS: &[(&str, &str)] = &[
    ("check_concurrency", "Checks at the same time"),
    ("rl_ip_per_min", "Visits per minute from one address"),
    ("rl_key_per_min", "API calls per minute from one key"),
    ("rl_ping_per_s", "Pings per second for one heartbeat"),
    ("tickets_per_ref_per_day", "Bug reports per user per day"),
    (
        "tickets_per_project_per_day",
        "Bug reports per project per day",
    ),
];

/// The largest value a limit accepts. A value above `u32::MAX` would wrap in the limiter.
pub fn max_for(key: &str) -> i64 {
    if key == "check_concurrency" {
        500
    } else {
        1_000_000
    }
}

pub async fn get(conn: &Connection, key: &str) -> anyhow::Result<i64> {
    let raw = {
        let mut rows = conn
            .query("SELECT value FROM settings WHERE key = ?1", [key])
            .await?;
        match rows.next().await? {
            Some(row) => row.get::<String>(0)?,
            None => anyhow::bail!("unknown setting {key}"),
        }
    };
    Ok(raw.parse()?)
}

/// `get`, or `default` when the read fails. The failure is logged, never swallowed.
pub async fn get_or(conn: &Connection, key: &str, default: i64) -> i64 {
    get(conn, key).await.unwrap_or_else(|e| {
        tracing::warn!(key, error = %e, "setting unreadable; using the default");
        default
    })
}

pub async fn all(conn: &Connection) -> anyhow::Result<Vec<(String, i64)>> {
    let mut out = Vec::new();
    for (key, _) in KEYS {
        out.push((key.to_string(), get(conn, key).await?));
    }
    Ok(out)
}

pub async fn set(conn: &Connection, key: &str, value: i64) -> anyhow::Result<()> {
    anyhow::ensure!(KEYS.iter().any(|(k, _)| *k == key), "unknown setting {key}");
    anyhow::ensure!(
        (1..=max_for(key)).contains(&value),
        "{key} must be between 1 and {}",
        max_for(key)
    );
    conn.execute(
        "UPDATE settings SET value = ?1 WHERE key = ?2",
        (value.to_string(), key),
    )
    .await?;
    Ok(())
}

/// Internal bookkeeping such as `last_rollup_day`. Not in `KEYS`, so the limits page never shows it.
pub async fn get_raw(conn: &Connection, key: &str) -> anyhow::Result<Option<String>> {
    let mut rows = conn
        .query("SELECT value FROM settings WHERE key = ?1", [key])
        .await?;
    Ok(match rows.next().await? {
        Some(row) => Some(row.get::<String>(0)?),
        None => None,
    })
}

pub async fn set_raw(conn: &Connection, key: &str, value: &str) -> anyhow::Result<()> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT (key) DO UPDATE SET value = excluded.value",
        (key, value),
    )
    .await?;
    Ok(())
}
