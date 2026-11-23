//! The summary report of one project over a range of days (v1.2 Q56, Q57): availability per
//! monitor and heartbeat, the incidents, and the maintenance windows.

mod render;

pub use render::render;

use super::period::Range;
use crate::{
    incidents::{ItemRef, SourceKind, availability},
    platform::db::Db,
    projects::Project,
};

/// One monitor or heartbeat over the range.
pub struct SourceLine {
    pub kind: SourceKind,
    pub name: String,
    pub availability: Option<f64>,
    pub downtime_s: i64,
    pub incidents: i64,
    pub longest_s: Option<i64>,
    /// The seconds it existed in the range: the weight in the project's availability.
    pub span_s: i64,
}

pub struct IncidentLine {
    pub title: String,
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
    /// The incidents open at some time in the range, newest first, at most 200.
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
            name: name.clone(),
            availability: p.availability,
            downtime_s: p.downtime_s,
            incidents: p.incidents,
            longest_s: p.longest_s,
            span_s: (range.to - range.from.max(created_at)).max(0),
        });
    }
    let incidents =
        crate::incidents::list(db, project.id, None, (Some(range.from), Some(range.to)))
            .await?
            .into_iter()
            .map(|i| IncidentLine {
                title: i.title,
                severity: i.severity,
                automatic: i.origin == "auto",
                started_at: i.started_at,
                resolved_at: i.resolved_at,
                excluded: i.excluded,
            })
            .collect();
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

/// The project's availability: every system's time adds up, as on the project card.
pub fn overall(sources: &[SourceLine]) -> Option<f64> {
    let span: i64 = sources.iter().map(|s| s.span_s).sum();
    let down: i64 = sources.iter().map(|s| s.downtime_s.min(s.span_s)).sum();
    (span > 0).then(|| 100.0 * (1.0 - down as f64 / span as f64))
}

/// The mean time from start to resolve of the outages resolved in the range.
pub fn mean_to_resolve(incidents: &[IncidentLine], range: Range) -> Option<i64> {
    let done: Vec<i64> = incidents
        .iter()
        .filter(|i| i.outage())
        .filter_map(|i| i.resolved_at.map(|r| (i.started_at, r)))
        .filter(|&(_, r)| r >= range.from && r < range.to)
        .map(|(s, r)| r - s)
        .collect();
    (!done.is_empty()).then(|| done.iter().sum::<i64>() / done.len() as i64)
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    fn source(down: i64, span: i64) -> SourceLine {
        SourceLine {
            kind: SourceKind::Monitor,
            name: "API".into(),
            availability: None,
            downtime_s: down,
            incidents: 0,
            longest_s: None,
            span_s: span,
        }
    }

    pub(super) fn incident(
        started: i64,
        resolved: Option<i64>,
        excluded: Option<&str>,
    ) -> IncidentLine {
        IncidentLine {
            title: "API is down".into(),
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
    fn mean_time_to_resolve_counts_resolves_inside_the_range_only() {
        let r = Range {
            from: 1_000,
            to: 2_000,
        };
        let list = [
            incident(900, Some(1_100), None),   // 200 s, resolved inside
            incident(1_200, Some(1_600), None), // 400 s, resolved inside
            incident(1_500, None, None),        // open
            incident(1_800, Some(2_500), None), // resolved after the range
            incident(1_300, Some(1_400), Some("planned")),
            IncidentLine {
                automatic: false,
                ..incident(1_100, Some(1_900), None)
            },
        ];
        assert_eq!(
            mean_to_resolve(&list, r),
            Some(300),
            "no manual or reclassified incident"
        );
        assert_eq!(mean_to_resolve(&list[2..3], r), None);
    }
}
