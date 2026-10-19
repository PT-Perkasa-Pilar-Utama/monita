//! The monitors list: the last 24 hours of every monitor as a sparkline, its uptime and its mean
//! response time, and one line that sums up the project.

use std::collections::HashMap;

use super::{CheckRow, chart::Bucket, heartbeat::Heartbeat, store::charts as store};
use crate::platform::{db::Db, time};

pub const HOURS: usize = 24;
/// The sparkline's viewBox is `0 0 W H`.
pub const W: f64 = 160.0;
pub const H: f64 = 32.0;

/// One monitor's last 24 hours.
#[derive(Debug, Default, serde::Serialize)]
pub struct Day {
    /// SVG path data; a gap where an hour had no checks.
    pub path: String,
    /// x of each hour with a failed check.
    pub fails: Vec<f64>,
    pub uptime: Option<String>,
    pub mean_ms: Option<i64>,
}

/// The interval in words, for the form's select.
pub fn interval_words(s: i64) -> &'static str {
    match s {
        30 => "30 seconds",
        60 => "1 minute",
        180 => "3 minutes",
        300 => "5 minutes",
        _ => "?",
    }
}

pub fn interval_label(s: i64) -> &'static str {
    match s {
        30 => "30s",
        60 => "1m",
        180 => "3m",
        300 => "5m",
        _ => "?",
    }
}

pub fn check_summary(c: &CheckRow) -> String {
    let latency = c.latency_ms.map(|l| format!("{l}ms")).unwrap_or_default();
    match (c.status_code, &c.error) {
        (Some(code), _) => format!("{code} in {latency}"),
        (None, Some(e)) => e.chars().take(60).collect(),
        (None, None) => latency,
    }
}

pub async fn last_day(db: &Db, project_id: i64, now: i64) -> anyhow::Result<HashMap<i64, Day>> {
    let from = now - 3600 * HOURS as i64;
    let rows = store::project_buckets(&db.read().await?, project_id, from, 3600, HOURS).await?;
    let mut by_monitor: HashMap<i64, Vec<Bucket>> = HashMap::new();
    for (id, b) in rows {
        by_monitor.entry(id).or_default().push(b);
    }
    Ok(by_monitor
        .into_iter()
        .map(|(id, buckets)| (id, day(&buckets)))
        .collect())
}

/// Hours with checks become points, scaled to the slowest hour; an hour without checks breaks
/// the line. Uptime is good checks over all checks.
pub fn day(buckets: &[Bucket]) -> Day {
    let step = W / (HOURS - 1) as f64;
    let max = buckets
        .iter()
        .filter_map(|b| b.avg_ms)
        .max()
        .unwrap_or(0)
        .max(1) as f64;
    let mut path = String::new();
    let mut prev: Option<usize> = None;
    for b in buckets.iter().filter(|b| b.index < HOURS) {
        let Some(ms) = b.avg_ms else { continue };
        let x = b.index as f64 * step;
        let y = H - 2.0 - (ms as f64 / max) * (H - 4.0);
        let joined = prev.is_some_and(|p| p + 1 == b.index);
        path.push_str(&format!("{}{x:.1} {y:.1} ", if joined { "L" } else { "M" }));
        prev = Some(b.index);
    }
    let total: i64 = buckets.iter().map(|b| b.total).sum();
    let ok: i64 = buckets.iter().map(|b| b.ok).sum();
    let weighted: i64 = buckets
        .iter()
        .filter_map(|b| b.avg_ms.map(|ms| ms * b.total))
        .sum();
    Day {
        path: path.trim_end().to_string(),
        fails: buckets
            .iter()
            .filter(|b| b.ok < b.total && b.index < HOURS)
            .map(|b| b.index as f64 * step)
            .collect(),
        uptime: (total > 0).then(|| format!("{:.2}%", ok as f64 * 100.0 / total as f64)),
        mean_ms: (total > 0).then(|| weighted / total),
    }
}

/// The project's state in one sentence, from the state of each monitor or heartbeat.
pub fn summary(states: &[&str], noun: &str) -> (&'static str, String) {
    let n = states.len();
    let count = |s: &str| states.iter().filter(|x| **x == s).count();
    let (down, issue) = (count("down"), count("issue"));
    let noun = |k: usize| {
        if k == 1 {
            noun.to_string()
        } else {
            format!("{noun}s")
        }
    };
    if down > 0 {
        let verb = if down == 1 { "is" } else { "are" };
        ("down", format!("{down} of {n} {} {verb} down.", noun(n)))
    } else if issue > 0 {
        let verb = if issue == 1 { "has" } else { "have" };
        (
            "issue",
            format!("{issue} of {n} {} {verb} an open issue.", noun(n)),
        )
    } else if count("unknown") == n {
        (
            "unknown",
            format!("Waiting for the first check of {n} {}.", noun(n)),
        )
    } else {
        ("up", format!("All {n} {} are up.", noun(n)))
    }
}

/// How far a heartbeat is from its deadline: the share of the wait used, 0 to 100, and a sentence.
#[derive(serde::Serialize)]
pub struct Due {
    pub used: i64,
    pub late: bool,
    pub text: String,
}

pub fn due(h: &Heartbeat, now: i64) -> Due {
    let from = h.last_ping_at.unwrap_or(h.created_at);
    match h.deadline_at {
        Some(deadline) if deadline > now => Due {
            used: ((now - from) * 100 / (deadline - from).max(1)).clamp(0, 100),
            late: false,
            text: format!("Due within {}", time::duration(deadline - now)),
        },
        Some(deadline) => Due {
            used: 100,
            late: true,
            text: format!("Late by {}", time::duration(now - deadline)),
        },
        None => Due {
            used: 100,
            late: true,
            text: "Missed its deadline".into(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b(index: usize, total: i64, ok: i64, avg_ms: i64) -> Bucket {
        Bucket {
            index,
            total,
            ok,
            avg_ms: Some(avg_ms),
        }
    }

    #[test]
    fn an_hour_without_checks_breaks_the_line() {
        let d = day(&[b(0, 2, 2, 100), b(1, 2, 2, 50), b(3, 2, 2, 100)]);
        assert_eq!(d.path.matches('M').count(), 2);
        assert_eq!(d.path.matches('L').count(), 1);
    }

    #[test]
    fn uptime_and_mean_weigh_every_check_and_failed_hours_are_marked() {
        let d = day(&[b(0, 3, 3, 100), b(5, 1, 0, 500)]);
        assert_eq!(d.uptime.as_deref(), Some("75.00%"));
        assert_eq!(d.mean_ms, Some(200));
        assert_eq!(d.fails.len(), 1);
    }

    #[test]
    fn no_checks_gives_no_uptime() {
        let d = day(&[]);
        assert!(d.path.is_empty() && d.uptime.is_none() && d.mean_ms.is_none());
    }

    #[test]
    fn the_summary_names_the_worst_state_first() {
        assert_eq!(
            summary(&["up", "down", "issue"], "monitor").1,
            "1 of 3 monitors is down."
        );
        assert_eq!(
            summary(&["up", "issue"], "monitor").1,
            "1 of 2 monitors has an open issue."
        );
        assert_eq!(
            summary(&["up", "unknown"], "monitor").1,
            "All 2 monitors are up."
        );
        assert_eq!(
            summary(&["unknown"], "heartbeat").1,
            "Waiting for the first check of 1 heartbeat."
        );
    }

    fn hb(last: Option<i64>, deadline: Option<i64>) -> Heartbeat {
        Heartbeat {
            id: 1,
            project_id: 1,
            name: "Backup".into(),
            token: "t".into(),
            period_s: 3600,
            grace_s: 600,
            severity: "warning".into(),
            is_public: false,
            alert: true,
            auto_incident: true,
            last_ping_at: last,
            deadline_at: deadline,
            created_at: 0,
        }
    }

    #[test]
    fn a_waiting_heartbeat_shows_the_share_of_its_wait_used() {
        let d = due(&hb(Some(1000), Some(1000 + 4200)), 1000 + 2100);
        assert_eq!((d.used, d.late), (50, false));
        assert_eq!(d.text, "Due within 35m 0s");
    }

    #[test]
    fn a_passed_or_cleared_deadline_is_late() {
        let d = due(&hb(Some(0), Some(100)), 160);
        assert_eq!(
            (d.used, d.late, d.text.as_str()),
            (100, true, "Late by 1m 0s")
        );
        assert!(due(&hb(Some(0), None), 160).late);
    }
}
