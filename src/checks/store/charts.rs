//! Bucketed reads for the charts. Each is a `GROUP BY` over a fixed window, so it returns at most
//! one row per bucket, whatever the check interval or ping rate.

use crate::platform::sql::Connection;

use crate::{checks::chart::Bucket, incidents::SourceKind};

/// The width of one row of `check_slots` and `ping_slots`, in seconds.
pub const SLOT_S: i64 = 900;

/// Buckets that hold whole slots read the slot tables; finer ones (the hour chart) read raw rows.
fn on_slots(from: i64, step: i64) -> bool {
    from % SLOT_S == 0 && step % SLOT_S == 0
}

const SLOT_CHECKS: &str = "SELECT (slot * 900 - ?2) / ?3 AS b, sum(total), sum(good), CASE WHEN sum(timed) > 0 THEN sum(ms_sum) / sum(timed) END
    FROM check_slots WHERE monitor_id = ?1 AND slot >= ?2 / 900 AND slot < ?4 / 900 GROUP BY b ORDER BY b LIMIT ?5";
const RAW_CHECKS: &str = "SELECT (checked_at - ?2) / ?3 AS b, count(*), sum(ok OR in_window), CAST(avg(latency_ms) AS INTEGER) FROM check_results
    WHERE monitor_id = ?1 AND location_id IS NULL AND checked_at >= ?2 AND checked_at < ?4 GROUP BY b ORDER BY b LIMIT ?5";
const SLOT_PINGS: &str = "SELECT (slot * 900 - ?2) / ?3 AS b, sum(total), sum(good), NULL FROM ping_slots
    WHERE heartbeat_id = ?1 AND slot >= ?2 / 900 AND slot < ?4 / 900 GROUP BY b ORDER BY b LIMIT ?5";
const RAW_PINGS: &str = "SELECT (received_at - ?2) / ?3 AS b, count(*), sum(kind = 'ok' OR in_window), NULL FROM pings
    WHERE heartbeat_id = ?1 AND received_at >= ?2 AND received_at < ?4 GROUP BY b ORDER BY b LIMIT ?5";

/// Checks per bucket of `step` seconds from `from`, for `buckets` buckets.
pub async fn check_buckets(
    c: &Connection,
    monitor_id: i64,
    from: i64,
    step: i64,
    buckets: usize,
) -> anyhow::Result<Vec<Bucket>> {
    let sql = if on_slots(from, step) {
        SLOT_CHECKS
    } else {
        RAW_CHECKS
    };
    read_buckets(c, sql, monitor_id, from, step, buckets).await
}

/// Pings per bucket of `step` seconds from `from`, for `buckets` buckets; `ok` counts OK pings.
pub async fn ping_buckets(
    c: &Connection,
    heartbeat_id: i64,
    from: i64,
    step: i64,
    buckets: usize,
) -> anyhow::Result<Vec<Bucket>> {
    let sql = if on_slots(from, step) {
        SLOT_PINGS
    } else {
        RAW_PINGS
    };
    read_buckets(c, sql, heartbeat_id, from, step, buckets).await
}

async fn read_buckets(
    c: &Connection,
    sql: &str,
    id: i64,
    from: i64,
    step: i64,
    buckets: usize,
) -> anyhow::Result<Vec<Bucket>> {
    let until = from + step * buckets as i64;
    let params = (id, from, step, until, buckets as i64);
    crate::platform::db::query_all(c, sql, params, |r| {
        Ok(Bucket {
            index: r.get::<i64>(0)? as usize,
            total: r.get(1)?,
            ok: r.get(2)?,
            avg_ms: r.get(3)?,
        })
    })
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

/// Checks per bucket for every monitor of a project, as (monitor id, bucket). One row per monitor
/// and bucket, so at most `MONITORS_PER_PROJECT * buckets` rows. Each monitor is read on its own
/// through the slot table's key.
pub async fn project_buckets(
    c: &Connection,
    project_id: i64,
    from: i64,
    step: i64,
    buckets: usize,
) -> anyhow::Result<Vec<(i64, Bucket)>> {
    let ids = crate::platform::db::query_all(
        c,
        "SELECT id FROM monitors WHERE project_id = ?1 ORDER BY id LIMIT ?2",
        (project_id, crate::platform::caps::MONITORS_PER_PROJECT),
        |r| r.get::<i64>(0),
    )
    .await?;
    let mut out = Vec::new();
    for id in ids {
        let rows = check_buckets(c, id, from, step, buckets).await?;
        out.extend(rows.into_iter().map(|b| (id, b)));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::db::Db;

    /// The slot tables, filled by their triggers, give the same buckets as the raw rows: location
    /// rows left out, a failure in a window counted good, a check without a latency left out of
    /// the mean.
    #[tokio::test]
    async fn slot_buckets_match_the_raw_rows() {
        let dir = tempfile::tempdir().unwrap();
        let db = Db::open(&dir.path().join("t.db")).await.unwrap();
        crate::platform::migrate::run(&db).await.unwrap();
        let from = 1_800_000_000 - 1_800_000_000 % 86_400;
        db.write_without_fk(async |c| {
            for i in 0..2_000_i64 {
                let at = from + i * 43;
                let latency = (i % 9 != 0).then_some(20 + i % 300);
                let location = (i % 11 == 0).then_some(1_i64);
                c.execute(
                    "INSERT INTO check_results (monitor_id, checked_at, ok, latency_ms, location_id, in_window) VALUES (1, ?1, ?2, ?3, ?4, ?5)",
                    (at, i % 7 != 0, latency, location, i % 13 == 0),
                )
                .await?;
                c.execute(
                    "INSERT INTO pings (heartbeat_id, received_at, kind, in_window) VALUES (1, ?1, ?2, ?3)",
                    (at, if i % 5 == 0 { "fail" } else { "ok" }, i % 17 == 0),
                )
                .await?;
            }
            Ok(())
        })
        .await
        .unwrap();
        let c = db.read().await.unwrap();
        for (step, n) in [(900, 96), (3_600, 24), (86_400, 1)] {
            for (slot, raw) in [(SLOT_CHECKS, RAW_CHECKS), (SLOT_PINGS, RAW_PINGS)] {
                let a = read_buckets(&c, slot, 1, from, step, n).await.unwrap();
                let b = read_buckets(&c, raw, 1, from, step, n).await.unwrap();
                assert!(!a.is_empty());
                assert_eq!(a, b, "step {step}");
            }
        }
    }
}
