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

pub async fn monitor(
    db: &Db,
    m: &Monitor,
    range: Range,
    now: i64,
    off: i64,
) -> anyhow::Result<Series> {
    let w = Window::new(range, now, off);
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

pub async fn heartbeat(
    db: &Db,
    hb: &Heartbeat,
    range: Range,
    now: i64,
    off: i64,
) -> anyhow::Result<Series> {
    let w = Window::new(range, now, off);
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

/// The Overview's chart (v1.1 Q39): every monitor and heartbeat of the project over the last day,
/// in 15-minute slots. Paused ones read nothing; the chart counts them apart.
pub async fn project(
    db: &Db,
    project_id: i64,
    now: i64,
    off: i64,
) -> anyhow::Result<chart::overview::OverviewChart> {
    use chart::overview::{BeatSeries, MonitorSeries, overview};
    let w = Window::new(Range::Day, now, off);
    let states = crate::incidents::team_states_for(db, project_id).await?;
    let down = history::intervals(db, project_id, w.from, now).await?;
    let down_of = |kind, id| down.get(&(kind, id)).cloned().unwrap_or_default();
    let monitors = super::list_monitors(db, project_id).await?;
    let beats = super::heartbeat::list(db, project_id).await?;
    let c = db.read().await?;
    let mut ms = Vec::with_capacity(monitors.len());
    for m in monitors {
        let incident = states.get(&(SourceKind::Monitor, m.id)).copied();
        let state = super::item_state(m.last.is_some(), m.paused_at.is_some(), incident).0;
        let buckets = if state == "paused" {
            Vec::new()
        } else {
            store::check_buckets(&c, m.id, w.from, w.step, w.buckets).await?
        };
        ms.push(MonitorSeries {
            id: m.id,
            state,
            interval_s: m.interval_s,
            buckets,
            down: down_of(SourceKind::Monitor, m.id),
            name: m.name,
        });
    }
    let mut bs = Vec::with_capacity(beats.len());
    for h in beats {
        let state = h
            .state(states.get(&(SourceKind::Heartbeat, h.id)).copied())
            .0;
        let buckets = if state == "paused" {
            Vec::new()
        } else {
            store::ping_buckets(&c, h.id, w.from, w.step, w.buckets).await?
        };
        let since = h.last_ping_at.unwrap_or(h.created_at);
        bs.push(BeatSeries {
            id: h.id,
            state,
            buckets,
            down: down_of(SourceKind::Heartbeat, h.id),
            due: (now - since) * 100 / (h.period_s + h.grace_s).max(1),
            name: h.name,
        });
    }
    Ok(overview(&w, ms, bs))
}

/// The project list's chart (v1.1 Q39): one series per project of the page. A project's line is
/// the mean response time of its monitors per slot, weighted by checks, and its red shading is
/// its monitors' automatic incidents. A project with heartbeats also gets a lane: their pings
/// together, and the time they missed or failed. Paused monitors and heartbeats read nothing.
// ponytail: one bucket read per monitor, PROJECTS_PAGE * MONITORS_PER_PROJECT (2000) at most a
// read; move to an hourly rollup written with each check if a full page reads slowly.
pub async fn projects(
    db: &Db,
    projects: &[(i64, String)],
    now: i64,
    off: i64,
) -> anyhow::Result<chart::overview::OverviewChart> {
    use chart::overview::{BeatSeries, MonitorSeries, overview, rank};
    let w = Window::new(Range::Day, now, off);
    let mut series = Vec::with_capacity(projects.len());
    let mut lanes = Vec::new();
    for (id, name) in projects {
        let states = crate::incidents::team_states_for(db, *id).await?;
        let down = history::intervals(db, *id, w.from, now).await?;
        let monitors = super::list_monitors(db, *id).await?;
        let beats = super::heartbeat::list(db, *id).await?;
        let mut state = "unknown";
        let mut worse = |s: &'static str| {
            if rank(s) < rank(state) {
                state = s;
            }
        };
        let c = db.read().await?;
        let mut sums: Vec<(i64, i64, i64, i64)> = vec![(0, 0, 0, 0); w.buckets];
        let mut interval_s = 300;
        for m in &monitors {
            let incident = states.get(&(SourceKind::Monitor, m.id)).copied();
            worse(super::item_state(m.last.is_some(), m.paused_at.is_some(), incident).0);
            if m.paused_at.is_some() {
                continue;
            }
            interval_s = interval_s.min(m.interval_s);
            for b in store::check_buckets(&c, m.id, w.from, w.step, w.buckets).await? {
                let s = &mut sums[b.index.min(w.buckets - 1)];
                s.0 += b.total;
                s.1 += b.ok;
                if let Some(v) = b.avg_ms {
                    s.2 += v * b.total;
                    s.3 += b.total;
                }
            }
        }
        let mut beat_state = "unknown";
        let mut pings: Vec<(i64, i64)> = vec![(0, 0); w.buckets];
        let mut due = 0;
        for h in &beats {
            let s = h
                .state(states.get(&(SourceKind::Heartbeat, h.id)).copied())
                .0;
            worse(s);
            if rank(s) < rank(beat_state) {
                beat_state = s;
            }
            if h.paused_at.is_some() {
                continue;
            }
            let since = h.last_ping_at.unwrap_or(h.created_at);
            due = due.max((now - since) * 100 / (h.period_s + h.grace_s).max(1));
            for b in store::ping_buckets(&c, h.id, w.from, w.step, w.buckets).await? {
                let p = &mut pings[b.index.min(w.buckets - 1)];
                p.0 += b.total;
                p.1 += b.ok;
            }
        }
        let down_of = |kind| {
            down.iter()
                .filter(|((k, _), _)| *k == kind)
                .flat_map(|(_, spans)| spans.iter().copied())
                .collect::<Vec<(i64, i64)>>()
        };
        if beats.iter().any(|h| h.paused_at.is_none()) {
            lanes.push(BeatSeries {
                id: *id,
                name: name.clone(),
                state: beat_state,
                buckets: pings
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| p.0 > 0)
                    .map(|(index, &(total, ok))| Bucket {
                        index,
                        total,
                        ok,
                        avg_ms: None,
                    })
                    .collect(),
                down: down_of(SourceKind::Heartbeat),
                due,
            });
        }
        let buckets = sums
            .into_iter()
            .enumerate()
            .filter(|(_, s)| s.0 > 0)
            .map(|(index, (total, ok, ms, timed))| Bucket {
                index,
                total,
                ok,
                avg_ms: (timed > 0).then(|| ms / timed),
            })
            .collect();
        // A project whose every system is paused reads as paused and is left out of the lines.
        let any = !monitors.is_empty() || !beats.is_empty();
        if any
            && monitors.iter().all(|m| m.paused_at.is_some())
            && beats.iter().all(|h| h.paused_at.is_some())
        {
            state = "paused";
        }
        series.push(MonitorSeries {
            id: *id,
            name: name.clone(),
            state,
            interval_s,
            buckets,
            down: down_of(SourceKind::Monitor),
        });
    }
    Ok(overview(&w, series, lanes))
}
