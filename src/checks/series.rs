//! The data behind the monitor and heartbeat charts: bucketed reads and down spans, turned into
//! geometry by `chart`. Hour, day and week read the raw rows (kept 7 days); month reads the daily
//! rollup and today's raw rows.

use super::{
    Monitor,
    chart::{self, Bucket, Chart, Range, Window},
    heartbeat::Heartbeat,
    store::charts as store,
};
use crate::{
    incidents::{ItemRef, SourceKind, history},
    platform::{db::Db, time},
};

pub struct Series {
    pub chart: Chart,
    pub range: Range,
    /// Checks or pings in the range, and how many were good.
    pub total: i64,
    pub ok: i64,
    /// Mean response time over the range, weighted by checks.
    pub mean_ms: Option<i64>,
}

pub async fn monitor(db: &Db, m: &Monitor, range: Range, now: i64) -> anyhow::Result<Series> {
    let w = Window::new(range, now);
    let c = db.read().await?;
    let buckets = match range {
        Range::Month => month(&c, SourceKind::Monitor, m.id, &w, now).await?,
        _ => store::check_buckets(&c, m.id, w.from, w.step, w.buckets).await?,
    };
    let item = ItemRef {
        kind: SourceKind::Monitor,
        id: m.id,
    };
    let down = history::spans(&c, item, w.from, w.end(), now).await?;
    // Bridge the empty buckets between two checks; break where checks stopped.
    let max_gap = (m.interval_s / w.step).max(1) as usize + 1;
    let chart = chart::latency(range, &w, &buckets, max_gap, m.created_at, now, &down);
    Ok(series(chart, range, &buckets))
}

pub async fn heartbeat(db: &Db, hb: &Heartbeat, range: Range, now: i64) -> anyhow::Result<Series> {
    let w = Window::new(range, now);
    let c = db.read().await?;
    let buckets = match range {
        Range::Month => month(&c, SourceKind::Heartbeat, hb.id, &w, now).await?,
        _ => store::ping_buckets(&c, hb.id, w.from, w.step, w.buckets).await?,
    };
    let item = ItemRef {
        kind: SourceKind::Heartbeat,
        id: hb.id,
    };
    let down = history::spans(&c, item, w.from, w.end(), now).await?;
    let chart = chart::pings(range, &w, &buckets, hb.created_at, now, &down);
    Ok(series(chart, range, &buckets))
}

/// One bucket per day: the rollup for the days before today, the raw rows for today.
async fn month(
    c: &turso::Connection,
    kind: SourceKind,
    id: i64,
    w: &Window,
    now: i64,
) -> anyhow::Result<Vec<Bucket>> {
    let days: Vec<String> = (0..w.buckets)
        .map(|i| time::day(w.from + i as i64 * w.step))
        .collect();
    let mut out: Vec<Bucket> = store::daily(c, kind, id, &days[0], w.buckets)
        .await?
        .into_iter()
        .filter_map(|(day, total, ok, avg_ms)| {
            let index = days.iter().position(|d| *d == day)?;
            (index + 1 < w.buckets).then_some(Bucket {
                index,
                total,
                ok,
                avg_ms,
            })
        })
        .collect();
    let today = time::day_start(now);
    let raw = match kind {
        SourceKind::Monitor => store::check_buckets(c, id, today, 86_400, 1).await?,
        SourceKind::Heartbeat => store::ping_buckets(c, id, today, 86_400, 1).await?,
    };
    out.extend(raw.into_iter().map(|b| Bucket {
        index: w.buckets - 1,
        ..b
    }));
    Ok(out)
}

fn series(chart: Chart, range: Range, buckets: &[Bucket]) -> Series {
    let total: i64 = buckets.iter().map(|b| b.total).sum();
    let ok = buckets.iter().map(|b| b.ok).sum();
    let timed: Vec<(i64, i64)> = buckets
        .iter()
        .filter_map(|b| b.avg_ms.map(|v| (v, b.total)))
        .collect();
    let weight: i64 = timed.iter().map(|(_, n)| n).sum();
    let mean_ms = (weight > 0).then(|| timed.iter().map(|(v, n)| v * n).sum::<i64>() / weight);
    Series {
        chart,
        range,
        total,
        ok,
        mean_ms,
    }
}
