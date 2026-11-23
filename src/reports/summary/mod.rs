//! The summary report of one project over a range of days (v1.2 Q56, Q57, Q62): availability per
//! day and per monitor and heartbeat, the maintenance, and each incident with what it touched.

mod render;

pub use render::render;

use super::period::Range;
use crate::{
    incidents::{ItemRef, SourceKind, availability},
    platform::{caps, db::Db},
    projects::Project,
};

/// One monitor or heartbeat over the range.
pub struct SourceLine {
    pub kind: SourceKind,
    pub id: i64,
    pub name: String,
    pub created_at: i64,
    pub availability: Option<f64>,
    pub downtime_s: i64,
    pub incidents: i64,
    /// Mean time to repair: the mean length of its outages.
    pub mttr_s: Option<i64>,
    /// The seconds it existed in the range: the weight in the project's availability.
    pub span_s: i64,
}

impl SourceLine {
    /// Mean time between failures: its time up in the range over its outages, as Site24x7
    /// reports it; `None` with no outage.
    pub fn mtbf_s(&self) -> Option<i64> {
        (self.incidents > 0).then(|| (self.span_s - self.downtime_s).max(0) / self.incidents)
    }
}

/// One day of the range: the share of all systems' time that was not down.
#[derive(Debug, PartialEq)]
pub struct DayLine {
    pub day: String,
    /// `None` when no system existed that day.
    pub availability: Option<f64>,
    pub downtime_s: i64,
}

pub struct IncidentLine {
    pub id: i64,
    pub title: String,
    pub cause: Option<String>,
    /// The monitors and heartbeats it touched, read only for the incidents the report lists.
    pub items: Vec<String>,
    /// The newest update: its label, its message and when.
    pub latest: Option<(String, String, i64)>,
    pub severity: String,
    /// Opened by Monita after failed checks, not by hand.
    pub automatic: bool,
    pub started_at: i64,
    pub resolved_at: Option<i64>,
    pub excluded: Option<String>,
}

impl IncidentLine {
    /// An outage counts as downtime: automatic and not reclassified (Q46, v1.2 Q57).
    pub fn outage(&self) -> bool {
        self.automatic && self.excluded.is_none()
    }
}

pub struct WindowLine {
    pub title: String,
    pub starts_at: i64,
    pub ends_at: i64,
}

pub struct Summary {
    pub project: String,
    pub range: Range,
    pub off: i64,
    pub now: i64,
    pub sources: Vec<SourceLine>,
    pub days: Vec<DayLine>,
    /// The incidents open at some time in the range, longest first, at most 200. Only the first
    /// `caps::REPORT_INCIDENTS` carry their items and latest update.
    pub incidents: Vec<IncidentLine>,
    pub windows: Vec<WindowLine>,
    pub windows_total: i64,
}

/// Reads the report's numbers through each module's interface.
pub async fn gather(
    db: &Db,
    project: &Project,
    range: Range,
    now: i64,
    off: i64,
) -> anyhow::Result<Summary> {
    let mut sources = Vec::new();
    let monitors = crate::checks::list_monitors(db, project.id).await?;
    let beats = crate::checks::heartbeat::list(db, project.id).await?;
    let all = monitors
        .iter()
        .map(|m| (SourceKind::Monitor, m.id, &m.name, m.created_at))
        .chain(
            beats
                .iter()
                .map(|h| (SourceKind::Heartbeat, h.id, &h.name, h.created_at)),
        );
    for (kind, id, name, created_at) in all {
        let p = availability::between(db, ItemRef { kind, id }, created_at, range.from, range.to)
            .await?;
        sources.push(SourceLine {
            kind,
            id,
            name: name.clone(),
            created_at,
            availability: p.availability,
            downtime_s: p.downtime_s,
            incidents: p.incidents,
            mttr_s: p.average_s,
            span_s: (range.to - range.from.max(created_at)).max(0),
        });
    }
    // The same outages as the numbers above: automatic, not reclassified (history.rs).
    let spans = crate::incidents::history::intervals(db, project.id, range.from, now).await?;
    let days = by_day(
        &sources
            .iter()
            .map(|s| {
                let key = (s.kind, s.id);
                (s.created_at, spans.get(&key).cloned().unwrap_or_default())
            })
            .collect::<Vec<_>>(),
        range,
        off,
    );
    let mut incidents: Vec<IncidentLine> =
        crate::incidents::list(db, project.id, None, (Some(range.from), Some(range.to)))
            .await?
            .into_iter()
            .map(|i| IncidentLine {
                id: i.id,
                title: i.title,
                cause: i.cause,
                items: Vec::new(),
                latest: None,
                severity: i.severity,
                automatic: i.origin == "auto",
                started_at: i.started_at,
                resolved_at: i.resolved_at,
                excluded: i.excluded,
            })
            .collect();
    incidents.sort_by_key(|i| std::cmp::Reverse(i.resolved_at.unwrap_or(now) - i.started_at));
    let names = crate::incidents::item_names(db, project.id).await?;
    for line in incidents.iter_mut().take(caps::REPORT_INCIDENTS as usize) {
        if let Some((_, refs, updates)) = crate::incidents::find(db, project.id, line.id).await? {
            line.items = refs
                .iter()
                .map(|r| {
                    names.iter().find(|(n, _)| n == r).map_or_else(
                        || format!("Deleted {}", r.kind.as_str()),
                        |(_, name)| name.clone(),
                    )
                })
                .collect();
            // Updates come newest first.
            line.latest = updates
                .into_iter()
                .next()
                .map(|u| (u.label, u.message, u.created_at));
        }
    }
    let filter = crate::maintenance::Filter {
        q: String::new(),
        state: None,
        oldest: true,
        from: Some(range.from),
        to: Some(range.to),
    };
    let (windows, windows_total) =
        crate::maintenance::page(db, project.id, 1, &filter, now).await?;
    Ok(Summary {
        project: project.name.clone(),
        range,
        off,
        now,
        sources,
        days,
        incidents,
        windows: windows
            .into_iter()
            .map(|w| WindowLine {
                title: w.title,
                starts_at: w.starts_at,
                ends_at: w.ends_at,
            })
            .collect(),
        windows_total,
    })
}

/// Each day of the range on the clock `off` minutes east of UTC. A system counts from when it
/// was added; its down time is the overlap of its outages with the day.
pub fn by_day(sources: &[(i64, Vec<(i64, i64)>)], range: Range, off: i64) -> Vec<DayLine> {
    let mut out = Vec::new();
    let mut lo = range.from;
    while lo < range.to {
        let hi = (lo + 86_400).min(range.to);
        let (mut watched, mut down) = (0_i64, 0_i64);
        for (created, spans) in sources {
            let start = lo.max(*created);
            if start >= hi {
                continue;
            }
            watched += hi - start;
            let d: i64 = spans
                .iter()
                .map(|&(s, e)| (e.min(hi) - s.max(start)).max(0))
                .sum();
            down += d.min(hi - start);
        }
        out.push(DayLine {
            day: crate::platform::time::day_at(lo, off),
            availability: (watched > 0).then(|| 100.0 * (1.0 - down as f64 / watched as f64)),
            downtime_s: down,
        });
        lo = hi;
    }
    out
}

/// Seconds inside the range when at least one listed window was active: windows that overlap
/// count their shared time once.
pub fn maintenance_s(windows: &[WindowLine], range: Range) -> i64 {
    let mut spans: Vec<(i64, i64)> = windows
        .iter()
        .map(|w| (w.starts_at.max(range.from), w.ends_at.min(range.to)))
        .filter(|(s, e)| e > s)
        .collect();
    spans.sort_unstable();
    let (mut total, mut reach) = (0, i64::MIN);
    for (s, e) in spans {
        let from = s.max(reach);
        if e > from {
            total += e - from;
        }
        reach = reach.max(e);
    }
    total
}

/// The project's availability: every system's time adds up, as on the project card.
pub fn overall(sources: &[SourceLine]) -> Option<f64> {
    let span: i64 = sources.iter().map(|s| s.span_s).sum();
    let down: i64 = sources.iter().map(|s| s.downtime_s.min(s.span_s)).sum();
    (span > 0).then(|| 100.0 * (1.0 - down as f64 / span as f64))
}

/// The mean length of an outage across the project, weighted by each system's outages, so it is
/// the same rule as each system's MTTR: an outage still open counts up to the range's end.
pub fn mean_repair(sources: &[SourceLine]) -> Option<i64> {
    let n: i64 = sources
        .iter()
        .filter(|s| s.mttr_s.is_some())
        .map(|s| s.incidents)
        .sum();
    let total: i64 = sources
        .iter()
        .filter_map(|s| s.mttr_s.map(|m| m * s.incidents))
        .sum();
    (n > 0).then(|| total / n)
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    fn source(down: i64, span: i64) -> SourceLine {
        SourceLine {
            kind: SourceKind::Monitor,
            id: 1,
            name: "API".into(),
            created_at: 0,
            availability: None,
            downtime_s: down,
            incidents: 0,
            mttr_s: None,
            span_s: span,
        }
    }

    pub(super) fn incident(
        started: i64,
        resolved: Option<i64>,
        excluded: Option<&str>,
    ) -> IncidentLine {
        IncidentLine {
            id: 1,
            title: "API is down".into(),
            cause: None,
            items: Vec::new(),
            latest: None,
            severity: "critical".into(),
            automatic: true,
            started_at: started,
            resolved_at: resolved,
            excluded: excluded.map(str::to_string),
        }
    }

    #[test]
    fn the_projects_availability_weighs_each_system_by_its_time() {
        // One system down 1 000 s of 10 000 s, another never down over 30 000 s.
        let a = overall(&[source(1_000, 10_000), source(0, 30_000)]).unwrap();
        assert!((a - 97.5).abs() < 1e-9, "{a}");
        assert_eq!(overall(&[]), None, "no systems, no data");
        assert_eq!(overall(&[source(0, 0)]), None, "added after the range");
    }

    #[test]
    fn the_projects_mttr_weighs_each_systems_mttr_by_its_outages() {
        let mut a = source(0, 10_000);
        (a.incidents, a.mttr_s) = (3, Some(100));
        let mut b = source(0, 10_000);
        (b.incidents, b.mttr_s) = (1, Some(500));
        assert_eq!(mean_repair(&[a, b, source(0, 10_000)]), Some(200));
        assert_eq!(mean_repair(&[source(0, 10_000)]), None, "no outage");
    }

    #[test]
    fn mean_time_between_failures_is_the_time_up_over_the_outages() {
        let mut s = source(600, 10_600);
        assert_eq!(s.mtbf_s(), None, "no outage");
        s.incidents = 2;
        assert_eq!(s.mtbf_s(), Some(5_000));
    }

    #[test]
    fn each_day_weighs_the_systems_that_existed_that_day() {
        // Two days from a UTC+7 midnight; one system all along, one added halfway through day 2.
        let off = 420;
        let from = crate::platform::time::parse_day_at("2026-10-01", off).unwrap();
        let r = Range {
            from,
            to: from + 2 * 86_400,
        };
        let days = by_day(
            &[
                (0, vec![(from + 3_600, from + 7_200)]),
                (from + 86_400 + 43_200, vec![]),
            ],
            r,
            off,
        );
        assert_eq!(days.len(), 2);
        assert_eq!(days[0].day, "2026-10-01");
        assert_eq!(days[0].downtime_s, 3_600);
        let a = days[0].availability.unwrap();
        assert!((a - 100.0 * (1.0 - 3_600.0 / 86_400.0)).abs() < 1e-9, "{a}");
        assert_eq!(days[1].availability, Some(100.0));
        assert_eq!(
            by_day(&[(r.to, vec![])], r, off)[0].availability,
            None,
            "added later"
        );
        let partial = Range {
            from,
            to: from + 3_600,
        };
        assert_eq!(
            by_day(&[(0, vec![])], partial, off).len(),
            1,
            "a cut day still counts"
        );
    }

    #[test]
    fn maintenance_counts_only_its_time_inside_the_range() {
        let r = Range {
            from: 1_000,
            to: 2_000,
        };
        let w = |s, e| WindowLine {
            title: "Move".into(),
            starts_at: s,
            ends_at: e,
        };
        assert_eq!(
            maintenance_s(&[w(900, 1_100), w(1_500, 1_600), w(1_900, 2_500)], r),
            300
        );
        assert_eq!(
            maintenance_s(&[w(1_200, 1_400), w(1_300, 1_500), w(1_350, 1_380)], r),
            300,
            "overlapping windows count once"
        );
    }
}
