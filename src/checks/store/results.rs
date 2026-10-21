//! SQL for check_results.

use turso::Connection;

use crate::checks::CheckResult;

#[derive(serde::Serialize)]
pub struct CheckRow {
    pub id: i64,
    pub checked_at: i64,
    pub ok: bool,
    pub status_code: Option<i64>,
    pub latency_ms: Option<i64>,
    pub error: Option<String>,
    /// The location that ran it, or None for this server (technical-specs 14).
    pub place: Option<String>,
}

/// `location_id` None: this server ran it.
pub async fn insert_check(
    c: &Connection,
    monitor_id: i64,
    location_id: Option<i64>,
    r: &CheckResult,
) -> anyhow::Result<()> {
    c.execute(
        "INSERT INTO check_results (monitor_id, checked_at, ok, status_code, latency_ms, error, location_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        (monitor_id, r.checked_at, r.ok, r.status_code.map(i64::from), r.latency_ms, r.error.clone(), location_id),
    )
    .await?;
    Ok(())
}

pub async fn recent_checks(
    c: &Connection,
    monitor_id: i64,
    since: i64,
    before: (i64, i64),
    limit: i64,
) -> anyhow::Result<Vec<CheckRow>> {
    crate::platform::db::query_all(
        c,
        "SELECT r.id, r.checked_at, r.ok, r.status_code, r.latency_ms, r.error, l.name
         FROM check_results r LEFT JOIN locations l ON l.id = r.location_id
         WHERE r.monitor_id = ?1 AND r.checked_at >= ?2 AND (r.checked_at < ?3 OR (r.checked_at = ?3 AND r.id < ?4))
         ORDER BY r.checked_at DESC, r.id DESC LIMIT ?5",
        (monitor_id, since, before.0, before.1, limit),
        |r| {
            Ok(CheckRow {
                id: r.get(0)?,
                checked_at: r.get(1)?,
                ok: r.get::<i64>(2)? == 1,
                status_code: r.get(3)?,
                latency_ms: r.get(4)?,
                error: r.get(5)?,
                place: r.get(6)?,
            })
        },
    )
    .await
}
