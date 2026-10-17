//! Bucketed reads for the charts. Each is a `GROUP BY` over a fixed window, so it returns at most
//! one row per bucket, whatever the check interval or ping rate.

use turso::Connection;

use crate::{checks::chart::Bucket, incidents::SourceKind};

/// Checks per bucket of `step` seconds from `from`, for `buckets` buckets.
pub async fn check_buckets(
    c: &Connection,
    monitor_id: i64,
    from: i64,
    step: i64,
    buckets: usize,
) -> anyhow::Result<Vec<Bucket>> {
    let until = from + step * buckets as i64;
    crate::platform::db::query_all(
        c,
        "SELECT (checked_at - ?2) / ?3 AS b, count(*), sum(ok), CAST(avg(latency_ms) AS INTEGER) FROM check_results
         WHERE monitor_id = ?1 AND checked_at >= ?2 AND checked_at < ?4 GROUP BY b ORDER BY b LIMIT ?5",
        (monitor_id, from, step, until, buckets as i64),
        |r| {
            Ok(Bucket {
                index: r.get::<i64>(0)? as usize,
                total: r.get(1)?,
                ok: r.get(2)?,
                avg_ms: r.get(3)?,
            })
        },
    )
    .await
}

/// Pings per bucket of `step` seconds from `from`, for `buckets` buckets; `ok` counts OK pings.
pub async fn ping_buckets(
    c: &Connection,
    heartbeat_id: i64,
    from: i64,
    step: i64,
    buckets: usize,
) -> anyhow::Result<Vec<Bucket>> {
    let until = from + step * buckets as i64;
    crate::platform::db::query_all(
        c,
        "SELECT (received_at - ?2) / ?3 AS b, count(*), sum(kind = 'ok') FROM pings
         WHERE heartbeat_id = ?1 AND received_at >= ?2 AND received_at < ?4 GROUP BY b ORDER BY b LIMIT ?5",
        (heartbeat_id, from, step, until, buckets as i64),
        |r| {
            Ok(Bucket {
                index: r.get::<i64>(0)? as usize,
                total: r.get(1)?,
                ok: r.get(2)?,
                avg_ms: None,
            })
        },
    )
    .await
}

/// The rolled-up days from `first_day` (`YYYY-MM-DD`) on: (day, total, ok, mean latency).
pub async fn daily(
    c: &Connection,
    kind: SourceKind,
    id: i64,
    first_day: &str,
    days: usize,
) -> anyhow::Result<Vec<(String, i64, i64, Option<i64>)>> {
    crate::platform::db::query_all(
        c,
        "SELECT day, checks_total, checks_ok, avg_latency_ms FROM daily_summaries
         WHERE source_kind = ?1 AND source_id = ?2 AND day >= ?3 ORDER BY day LIMIT ?4",
        (kind.as_str(), id, first_day, days as i64),
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    )
    .await
}
