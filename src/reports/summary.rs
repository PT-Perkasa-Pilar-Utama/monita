//! The summary report of one project over a range of days (v1.2 Q56, Q57): availability per
//! monitor and heartbeat, the incidents, and the maintenance windows.

use super::period::Range;
use super::sheet::{Col, Sheet};
use crate::{
    incidents::{ItemRef, SourceKind, availability},
    platform::{caps, db::Db, time},
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

fn percent(p: Option<f64>) -> String {
    p.map_or("No data".into(), |p| format!("{p:.2}%"))
}

fn state(i: &IncidentLine) -> &'static str {
    match (i.excluded.as_deref(), i.resolved_at) {
        (Some("planned"), _) => "Planned work",
        (Some(_), _) => "False alarm",
        (None, None) => "Open",
        (None, Some(_)) => "Resolved",
    }
}

fn title_case(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().chain(c).collect())
        .unwrap_or_default()
}

/// The PDF. `footer` names the project and the range on every page.
pub fn render(s: &Summary) -> anyhow::Result<Vec<u8>> {
    let first = time::day_at(s.range.from, s.off);
    let last = time::day_at(s.range.to - 1, s.off);
    let zone = time::zone(s.off);
    let mut sheet = Sheet::new(
        &format!("{} report, {first} to {last}", s.project),
        &format!("{}, {first} to {last}", s.project),
    );
    sheet.title(&format!("{} report", s.project));
    sheet.note(&format!(
        "This report covers {first} to {last}, and its times are in {zone}. Monita made it on {}.",
        time::stamp(s.now, s.off)
    ));

    let mttr = mean_to_resolve(&s.incidents, s.range);
    sheet.figures(&[
        ("Availability", percent(overall(&s.sources))),
        (
            "Downtime",
            time::duration(s.sources.iter().map(|x| x.downtime_s).sum()),
        ),
        (
            "Outages",
            s.incidents
                .iter()
                .filter(|i| i.outage())
                .count()
                .to_string(),
        ),
        (
            "Mean time to resolve",
            mttr.map_or("None resolved".into(), time::duration),
        ),
    ]);

    sheet.heading("Monitors and heartbeats");
    if s.sources.is_empty() {
        sheet.para("This project has no monitors or heartbeats.");
    } else {
        let rows: Vec<Vec<String>> = s
            .sources
            .iter()
            .map(|x| {
                vec![
                    x.name.clone(),
                    title_case(x.kind.as_str()),
                    percent(x.availability),
                    time::duration(x.downtime_s),
                    x.incidents.to_string(),
                    x.longest_s.map_or("None".into(), time::duration),
                ]
            })
            .collect();
        sheet.table(
            &[
                Col::left("Name", 0.34),
                Col::left("Kind", 0.12),
                Col::right("Availability", 0.15),
                Col::right("Downtime", 0.14),
                Col::right("Incidents", 0.11),
                Col::right("Longest", 0.14),
            ],
            &rows,
        );
    }

    sheet.heading("Incidents");
    if s.incidents.is_empty() {
        sheet.para("No incident was open in this period.");
    } else {
        let critical = s
            .incidents
            .iter()
            .filter(|i| i.severity == "critical")
            .count();
        let many = if s.incidents.len() as i64 >= caps::PROJECT_INCIDENTS {
            format!("At least {}", s.incidents.len())
        } else {
            s.incidents.len().to_string()
        };
        sheet.para(&format!(
            "{many} incidents were open in this period: {critical} critical and {} warning.",
            s.incidents.len() - critical
        ));
        let mut longest: Vec<&IncidentLine> = s.incidents.iter().collect();
        let length = |i: &IncidentLine| i.resolved_at.unwrap_or(s.now) - i.started_at;
        longest.sort_by_key(|i| std::cmp::Reverse(length(i)));
        let shown = caps::REPORT_INCIDENTS as usize;
        if longest.len() > shown {
            sheet.note(&format!(
                "The table shows the {shown} longest of them, longest first."
            ));
        }
        let rows: Vec<Vec<String>> = longest
            .iter()
            .take(shown)
            .map(|i| {
                vec![
                    time::local(i.started_at, s.off),
                    i.title.clone(),
                    title_case(&i.severity),
                    time::duration(length(i)),
                    state(i).to_string(),
                ]
            })
            .collect();
        sheet.table(
            &[
                Col::left("Started", 0.2),
                Col::wrapping("Incident", 0.39),
                Col::left("Alert level", 0.13),
                Col::right("Duration", 0.13),
                Col::left("State", 0.15),
            ],
            &rows,
        );
    }

    sheet.heading("Maintenance windows");
    if s.windows.is_empty() {
        sheet.para("No maintenance window overlapped this period.");
    } else {
        if s.windows_total > s.windows.len() as i64 {
            sheet.note(&format!(
                "The table shows the first {} of {} windows.",
                s.windows.len(),
                s.windows_total
            ));
        }
        let rows: Vec<Vec<String>> = s
            .windows
            .iter()
            .map(|w| {
                vec![
                    time::local(w.starts_at, s.off),
                    time::local(w.ends_at, s.off),
                    w.title.clone(),
                ]
            })
            .collect();
        sheet.table(
            &[
                Col::left("Starts", 0.22),
                Col::left("Ends", 0.22),
                Col::wrapping("Title", 0.56),
            ],
            &rows,
        );
    }

    sheet.heading("How Monita counts");
    sheet.note(
        "An outage is an incident that Monita opened after failed checks and nobody reclassified \
         as planned work or a false alarm. Availability is the share of each system's time \
         without an outage, counted from when the system was added, and the project's \
         availability adds up the time of all its systems. Manual incidents are not downtime, \
         and a failure inside a maintenance window opens no incident. The mean time to resolve \
         is the mean time from the start to the resolve of the outages resolved in this period.",
    );
    sheet.finish()
}

#[cfg(test)]
mod tests {
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

    fn incident(started: i64, resolved: Option<i64>, excluded: Option<&str>) -> IncidentLine {
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

    #[test]
    fn a_reclassified_incident_is_named_by_its_class() {
        assert_eq!(state(&incident(1, None, None)), "Open");
        assert_eq!(state(&incident(1, Some(2), None)), "Resolved");
        assert_eq!(
            state(&incident(1, Some(2), Some("planned"))),
            "Planned work"
        );
        assert_eq!(
            state(&incident(1, Some(2), Some("false_alarm"))),
            "False alarm"
        );
    }
}
