//! The summary report's layout on the page (v1.2 Q62): the top figures, availability by day,
//! each monitor and heartbeat, the maintenance, then each incident with what it touched. The
//! sections follow Site24x7's availability summary and Pingdom's uptime report.

use super::{DayLine, IncidentLine, Summary, maintenance_s, mean_repair, overall};
use crate::{
    platform::{caps, pdf::Rgb, time},
    reports::sheet::{Col, Sheet},
};

// The status page's day colours: no downtime, under 1 % down, more, and no data.
const UP: Rgb = Rgb(0.137, 0.471, 0.306);
const PARTIAL: Rgb = Rgb(0.949, 0.788, 0.298);
const DOWN: Rgb = Rgb(0.824, 0.271, 0.271);
const NONE: Rgb = Rgb(0.85, 0.86, 0.87);

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

/// `1 incident`, `3 incidents`.
fn count(n: usize, noun: &str) -> String {
    if n == 1 {
        format!("1 {noun}")
    } else {
        format!("{n} {noun}s")
    }
}

/// The colour of one day, as the status page draws it.
fn tone(d: &DayLine) -> Rgb {
    match d.availability {
        None => NONE,
        Some(_) if d.downtime_s == 0 => UP,
        Some(a) if a >= 99.0 => PARTIAL,
        Some(_) => DOWN,
    }
}

/// The PDF. The footer names the project and the range on every page.
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

    let capped = s.incidents.len() as i64 >= caps::PROJECT_INCIDENTS;
    let outages = s.incidents.iter().filter(|i| i.outage()).count();
    sheet.figures(&[
        ("Availability", percent(overall(&s.sources))),
        (
            "Downtime",
            time::duration(s.sources.iter().map(|x| x.downtime_s).sum()),
        ),
        (
            "Outages",
            if capped {
                format!("{outages} or more")
            } else {
                outages.to_string()
            },
        ),
        (
            "Mean time to resolve",
            mean_repair(&s.sources).map_or("None".into(), time::duration),
        ),
    ]);

    if !s.days.is_empty() {
        sheet.heading("Availability by day");
        let colours: Vec<Rgb> = s.days.iter().map(tone).collect();
        sheet.bars(&colours, &first, &last);
        sheet.note(
            "Each bar is one day. Green had no downtime, yellow was down for less than 1% of \
             the day, red for more, and grey had no monitor or heartbeat yet.",
        );
    }

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
                    x.mttr_s.map_or("None".into(), time::duration),
                    x.mtbf_s().map_or("None".into(), time::duration),
                ]
            })
            .collect();
        sheet.table(
            &[
                Col::wrapping("Name", 0.25),
                Col::left("Kind", 0.12),
                Col::right("Availability", 0.14),
                Col::right("Downtime", 0.12),
                Col::right("Outages", 0.11),
                Col::right("MTTR", 0.13),
                Col::right("MTBF", 0.13),
            ],
            &rows,
        );
        sheet.note(
            "MTTR is the mean time to repair, the mean length of an outage. MTBF is the mean \
             time between failures, the time up divided by the outages.",
        );
    }

    sheet.heading("Maintenance");
    if s.windows.is_empty() {
        sheet.para("No maintenance window overlapped this period.");
    } else {
        let held = maintenance_s(&s.windows, s.range);
        let share = 100.0 * held as f64 / (s.range.to - s.range.from).max(1) as f64;
        let more = s.windows_total > s.windows.len() as i64;
        sheet.para(&format!(
            "Planned maintenance took {}{}, {share:.2}% of the period, in {}{}.",
            if more { "at least " } else { "" },
            time::duration(held),
            count(s.windows_total as usize, "window"),
            if more {
                format!(", and the table shows the first {}", s.windows.len())
            } else {
                String::new()
            },
        ));
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

    sheet.heading("Incidents");
    if s.incidents.is_empty() {
        sheet.para("No incident was open in this period.");
        return sheet.finish();
    }
    let critical = s
        .incidents
        .iter()
        .filter(|i| i.severity == "critical")
        .count();
    sheet.para(&format!(
        "{}{} {} open in this period: {critical} critical and {} warning.",
        if capped { "At least " } else { "" },
        count(s.incidents.len(), "incident"),
        if s.incidents.len() == 1 {
            "was"
        } else {
            "were"
        },
        s.incidents.len() - critical
    ));
    let shown = caps::REPORT_INCIDENTS as usize;
    if s.incidents.len() > shown {
        sheet.note(&format!(
            "The report shows the {shown} longest, longest first."
        ));
    }
    for i in s.incidents.iter().take(shown) {
        let end = i.resolved_at.unwrap_or(s.now);
        sheet.subheading(&format!("INC-{}: {}", i.id, i.title));
        let mut facts = vec![
            (
                "Alert level",
                format!("{}, {}", title_case(&i.severity), state(i).to_lowercase()),
            ),
            (
                "When",
                format!(
                    "{} to {}, {}",
                    time::local(i.started_at, s.off),
                    i.resolved_at
                        .map_or("now".into(), |t| time::local(t, s.off)),
                    time::duration(end - i.started_at)
                ),
            ),
            (
                "Affects",
                if i.items.is_empty() {
                    "No monitor or heartbeat".into()
                } else {
                    i.items.join(", ")
                },
            ),
        ];
        if let Some(c) = i.cause.as_deref().filter(|c| !c.trim().is_empty()) {
            facts.push(("Cause", c.to_string()));
        }
        if let Some((label, message, at)) = &i.latest {
            facts.push((
                "Latest update",
                format!(
                    "{}, {}: {message}",
                    title_case(label),
                    time::local(*at, s.off)
                ),
            ));
        }
        sheet.facts(&facts);
    }
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

    #[test]
    fn counts_read_as_words_and_days_take_the_status_page_colours() {
        assert_eq!(count(1, "incident"), "1 incident");
        assert_eq!(count(3, "window"), "3 windows");
        let day = |a: Option<f64>, d| DayLine {
            day: "2026-10-01".into(),
            availability: a,
            downtime_s: d,
        };
        assert_eq!(tone(&day(None, 0)), NONE);
        assert_eq!(tone(&day(Some(100.0), 0)), UP);
        assert_eq!(tone(&day(Some(99.5), 400)), PARTIAL);
        assert_eq!(tone(&day(Some(90.0), 8_640)), DOWN);
    }
}
