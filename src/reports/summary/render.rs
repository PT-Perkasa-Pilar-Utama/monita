//! The summary report's layout on the page.

use super::{IncidentLine, Summary, mean_to_resolve, overall};
use crate::{
    platform::{caps, time},
    reports::sheet::{Col, Sheet},
};

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
            // Counted from the newest `caps::PROJECT_INCIDENTS` of the range (Known limits).
            {
                let n = s.incidents.iter().filter(|i| i.outage()).count();
                if s.incidents.len() as i64 >= caps::PROJECT_INCIDENTS {
                    format!("{n} or more")
                } else {
                    n.to_string()
                }
            },
        ),
        (
            "Mean time to resolve",
            mttr.map_or("None".into(), time::duration),
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
                Col::wrapping("Name", 0.34),
                Col::left("Kind", 0.12),
                Col::right("Availability", 0.15),
                Col::right("Downtime", 0.14),
                Col::right("Outages", 0.11),
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
    use super::super::tests::incident;
    use super::*;

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
