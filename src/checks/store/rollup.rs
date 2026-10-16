//! SQL for the daily rollup and the retention sweep.

use turso::Connection;

use crate::incidents::SourceKind;

pub async fn all_sources(c: &Connection) -> anyhow::Result<Vec<(SourceKind, i64)>> {
    let mut out = Vec::new();
    let mut rows = c
        .query(
            "SELECT 'monitor', id FROM monitors UNION ALL SELECT 'heartbeat', id FROM heartbeats",
            (),
        )
        .await?;
    while let Some(r) = rows.next().await? {
        if let Some(kind) = SourceKind::parse(&r.get::<String>(0)?) {
            out.push((kind, r.get(1)?));
        }
    }
    Ok(out)
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

pub async fn sweep(
    c: &Connection,
    raw_cutoff: i64,
    outbox_cutoff: i64,
    now: i64,
) -> anyhow::Result<()> {
    c.execute(
        "DELETE FROM check_results WHERE checked_at < ?1",
        [raw_cutoff],
    )
    .await?;
    c.execute("DELETE FROM pings WHERE received_at < ?1", [raw_cutoff])
        .await?;
    c.execute(
        "DELETE FROM outbox WHERE created_at < ?1 AND (sent_at IS NOT NULL OR failed_at IS NOT NULL)",
        [outbox_cutoff],
    )
    .await?;
    c.execute("DELETE FROM sessions WHERE expires_at < ?1", [now])
        .await?;
    c.execute(
        "DELETE FROM invites WHERE expires_at < ?1 OR (used_at IS NOT NULL AND used_at < ?2)",
        (now, raw_cutoff),
    )
    .await?;
    Ok(())
}
