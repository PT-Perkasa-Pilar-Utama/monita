//! A project at a glance for the project list: its worst state, the two state lines, open
//! incidents and tickets, and one bar an hour for the last 24 hours.

use serde::Serialize;

use crate::platform::{db::Db, now};

/// One hour of the project's last day. `uptime` is the share of its systems' time that was not
/// down; null when no monitor or heartbeat existed in that hour.
#[derive(Debug, PartialEq, Serialize)]
pub struct Hour {
    /// `14:00`, the hour's start in the admin's offset. Named `day` so the page draws it with
    /// the same bar as the status page.
    pub day: String,
    pub uptime: Option<f64>,
    pub downtime_s: i64,
}

#[derive(Serialize)]
pub struct Pulse {
    /// `up`, `down`, `issue` or `unknown`: the worse of the monitors' and heartbeats'.
    pub kind: &'static str,
    pub monitors: crate::checks::Summary,
    pub heartbeats: crate::checks::Summary,
    pub incidents: i64,
    pub tickets: i64,
    pub hours: Vec<Hour>,
}

const HOURS: i64 = 24;

/// 24 hours ending with the current one. Down seconds of every system add up, so two systems
/// down for the whole hour read 0% when the project has two, 50% when it has four. A system
/// counts while it was watched, from when it was added until it was paused: an hour when no
/// system was watched has no data.
pub fn hours(intervals: &[(i64, i64)], watched: &[(i64, i64)], now: i64, off: i64) -> Vec<Hour> {
    let local = now + off * 60;
    let current = local - local.rem_euclid(3600) - off * 60;
    (0..HOURS)
        .map(|i| {
            let lo = current - (HOURS - 1 - i) * 3600;
            let hi = (lo + 3600).min(now);
            let down: i64 = intervals
                .iter()
                .map(|&(s, e)| (e.min(hi) - s.max(lo)).max(0))
                .sum();
            let span: i64 = watched
                .iter()
                .map(|&(s, e)| (hi.min(e) - lo.max(s)).max(0))
                .sum();
            Hour {
                day: crate::platform::time::local(lo, off)[11..16].to_string(),
                uptime: (span > 0).then(|| {
                    (10_000.0 * (1.0 - down.min(span) as f64 / span as f64)).round() / 100.0
                }),
                downtime_s: down,
            }
        })
        .collect()
}

/// The share of `[lo, hi)` that a source was watched and not down, in percent; `None` when it
/// was not watched in that time.
pub fn share_up(intervals: &[(i64, i64)], watched: (i64, i64), lo: i64, hi: i64) -> Option<f64> {
    let (lo, hi) = (lo.max(watched.0), hi.min(watched.1));
    let span = hi - lo;
    if span <= 0 {
        return None;
    }
    let down: i64 = intervals
        .iter()
        .map(|&(s, e)| (e.min(hi) - s.max(lo)).max(0))
        .sum();
    Some((10_000.0 * (1.0 - down.min(span) as f64 / span as f64)).round() / 100.0)
}

fn worse(a: &'static str, b: &'static str) -> &'static str {
    let rank = |k| match k {
        "down" => 4,
        "issue" => 3,
        "maintenance" => 2,
        "up" => 1,
        _ => 0,
    };
    if rank(b) > rank(a) { b } else { a }
}

/// Each part read through its module's interface.
pub async fn pulse(db: &Db, project_id: i64) -> anyhow::Result<Pulse> {
    let t = now();
    let off = crate::platform::settings::utc_offset(&db.read().await?).await?;
    let [monitors, heartbeats] = crate::checks::summaries(db, project_id).await?;
    let spans: Vec<(i64, i64)> =
        crate::incidents::history::intervals(db, project_id, t - HOURS * 3600, t)
            .await?
            .into_values()
            .flatten()
            .collect();
    Ok(Pulse {
        kind: worse(monitors.kind, heartbeats.kind),
        hours: hours(
            &spans,
            &monitors
                .sources
                .iter()
                .chain(&heartbeats.sources)
                .map(|b| b.watched)
                .collect::<Vec<_>>(),
            t,
            off,
        ),
        incidents: crate::incidents::open_count(db, project_id).await?,
        tickets: crate::tickets::open_count(db, project_id).await?,
        monitors,
        heartbeats,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOON: i64 = 1_792_108_800 + 12 * 3600; // 2026-10-16 12:00 UTC

    #[test]
    fn each_hour_counts_the_share_of_system_time_that_was_down() {
        let now = NOON + 1800;
        // One of two systems down from 11:00 to 11:30: a quarter of that hour.
        let old = NOON - 2 * 86_400;
        let always = (old, i64::MAX);
        let h = hours(&[(NOON - 3600, NOON - 1800)], &[always, always], now, 0);
        assert_eq!(h.len(), 24);
        assert_eq!(h[23].day, "12:00", "the current hour is last");
        assert_eq!(h[22].day, "11:00");
        assert_eq!((h[22].uptime, h[22].downtime_s), (Some(75.0), 1800));
        assert_eq!(h[23].uptime, Some(100.0));
        assert_eq!(h[0].day, "13:00", "the day before");
        assert!(
            hours(&[], &[], now, 0).iter().all(|h| h.uptime.is_none()),
            "no systems, no data"
        );
        // Added at 10:30: no data before 10:00, a full bar from 10:00 on.
        let young = hours(&[], &[(NOON - 5400, i64::MAX)], now, 0);
        assert_eq!(young[20].uptime, None, "09:00");
        assert_eq!(young[21].uptime, Some(100.0), "10:00");
        // Paused at 11:00: no data from then on.
        let paused = hours(&[], &[(old, NOON - 3600)], now, 0);
        assert_eq!(paused[21].uptime, Some(100.0), "10:00");
        assert_eq!(paused[22].uptime, None, "11:00");
        assert_eq!(
            hours(&[], &[always], now, 420)[23].day,
            "19:00",
            "hours read in the offset"
        );
    }

    #[test]
    fn a_source_is_up_for_the_share_of_its_watched_time_not_down() {
        let day = (NOON - 86_400, NOON);
        assert_eq!(share_up(&[], (0, i64::MAX), day.0, day.1), Some(100.0));
        assert_eq!(
            share_up(
                &[(NOON - 3600, NOON)],
                (NOON - 7200, i64::MAX),
                day.0,
                day.1
            ),
            Some(50.0),
            "added two hours ago, down for the last one"
        );
        assert_eq!(
            share_up(&[], (NOON + 1, i64::MAX), day.0, day.1),
            None,
            "added later"
        );
        assert_eq!(
            share_up(&[], (0, day.0), day.0, day.1),
            None,
            "paused before"
        );
    }

    #[test]
    fn the_worse_state_wins() {
        assert_eq!(worse("up", "down"), "down");
        assert_eq!(worse("issue", "up"), "issue");
        assert_eq!(worse("maintenance", "issue"), "issue");
        assert_eq!(worse("up", "maintenance"), "maintenance");
        assert_eq!(worse("unknown", "up"), "up");
    }
}
