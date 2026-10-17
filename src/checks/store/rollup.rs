//! SQL for the daily rollup and the retention sweep.

use turso::Connection;

use crate::incidents::SourceKind;

/// The next `limit` monitor or heartbeat ids above `after`, in id order.
pub async fn source_ids(
    c: &Connection,
    kind: SourceKind,
    after: i64,
    limit: i64,
) -> anyhow::Result<Vec<i64>> {
    let sql = match kind {
        SourceKind::Monitor => "SELECT id FROM monitors WHERE id > ?1 ORDER BY id LIMIT ?2",
        SourceKind::Heartbeat => "SELECT id FROM heartbeats WHERE id > ?1 ORDER BY id LIMIT ?2",
    };
    crate::platform::db::query_all(c, sql, (after, limit), |r| r.get(0)).await
}

/// (total, ok, average latency) of a monitor's checks on the UTC day starting at `day`.
pub async fn check_counts(
    c: &Connection,
    id: i64,
    day: i64,
) -> anyhow::Result<(i64, i64, Option<i64>)> {
    let mut rows = c
        .query(
            "SELECT count(*), coalesce(sum(ok), 0), CAST(avg(latency_ms) AS INTEGER) FROM check_results WHERE monitor_id = ?1 AND checked_at >= ?2 AND checked_at < ?3",
            (id, day, day + 86_400),
        )
        .await?;
    let r = rows
        .next()
        .await?
        .ok_or_else(|| anyhow::anyhow!("aggregate returned no row"))?;
    Ok((r.get(0)?, r.get(1)?, r.get(2)?))
}

/// (total, ok, no latency) of a heartbeat's pings on the UTC day starting at `day`.
pub async fn ping_counts(
    c: &Connection,
    id: i64,
    day: i64,
) -> anyhow::Result<(i64, i64, Option<i64>)> {
    let mut rows = c
        .query(
            "SELECT count(*), coalesce(sum(kind = 'ok'), 0) FROM pings WHERE heartbeat_id = ?1 AND received_at >= ?2 AND received_at < ?3",
            (id, day, day + 86_400),
        )
        .await?;
    let r = rows
        .next()
        .await?
        .ok_or_else(|| anyhow::anyhow!("aggregate returned no row"))?;
    Ok((r.get(0)?, r.get(1)?, None))
}

#[allow(clippy::too_many_arguments)]
pub async fn upsert_summary(
    c: &Connection,
    kind: SourceKind,
    id: i64,
    day: &str,
    total: i64,
    ok: i64,
    down: i64,
    latency: Option<i64>,
) -> anyhow::Result<()> {
    c.execute(
        "INSERT INTO daily_summaries (source_kind, source_id, day, checks_total, checks_ok, downtime_s, avg_latency_ms) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT (source_kind, source_id, day) DO UPDATE SET checks_total = excluded.checks_total, checks_ok = excluded.checks_ok,
           downtime_s = excluded.downtime_s, avg_latency_ms = excluded.avg_latency_ms",
        (kind.as_str(), id, day, total, ok, down, latency),
    )
    .await?;
    Ok(())
}

/// Deletes up to 5 000 raw rows of each table older than `raw_cutoff`. Returns how many went.
pub async fn sweep_chunk(c: &Connection, raw_cutoff: i64) -> anyhow::Result<u64> {
    let results = c
        .execute(
            "DELETE FROM check_results WHERE rowid IN (SELECT rowid FROM check_results WHERE checked_at < ?1 LIMIT 5000)",
            [raw_cutoff],
        )
        .await?;
    let pings = c
        .execute(
            "DELETE FROM pings WHERE rowid IN (SELECT rowid FROM pings WHERE received_at < ?1 LIMIT 5000)",
            [raw_cutoff],
        )
        .await?;
    Ok(results + pings)
}
