//! The report of one incident (v1.2 Q56): what it was, which systems it touched, how long it
//! lasted, and every update, as the team's incident page shows them.

use super::sheet::{Col, Sheet};
use crate::{
    incidents::{IncidentRow, UpdateRow},
    platform::{db::Db, time},
    projects::Project,
};

pub struct IncidentReport {
    pub project: String,
    pub incident: IncidentRow,
    /// `Monitor: API`, or `Deleted monitor` for a system removed since.
    pub items: Vec<String>,
    pub updates: Vec<UpdateRow>,
    pub off: i64,
    pub now: i64,
}

/// `None` when the project has no incident with this number.
pub async fn gather(
    db: &Db,
    project: &Project,
    id: i64,
    now: i64,
    off: i64,
) -> anyhow::Result<Option<IncidentReport>> {
    let Some((incident, refs, updates)) = crate::incidents::find(db, project.id, id).await? else {
        return Ok(None);
    };
    let names = crate::incidents::item_names(db, project.id).await?;
    let items = refs
        .iter()
        .map(|r| {
            let kind = r.kind.as_str();
            names.iter().find(|(n, _)| n == r).map_or_else(
                || format!("Deleted {kind}"),
                |(_, name)| format!("{}{}: {name}", kind[..1].to_uppercase(), &kind[1..]),
            )
        })
        .collect();
    Ok(Some(IncidentReport {
        project: project.name.clone(),
        incident,
        items,
        updates,
        off,
        now,
    }))
}

/// Who wrote an update: a person's email, an API key, or Monita itself.
fn author(u: &UpdateRow) -> String {
    match (&u.author_email, &u.author_key) {
        (Some(email), _) => email.clone(),
        (None, Some(key)) => format!("API key {key}"),
        (None, None) => "Monita".into(),
    }
}

fn class(i: &IncidentRow, off: i64) -> String {
    let by = |what: &str| match (&i.excluded_by, i.excluded_at) {
        (Some(who), Some(at)) => format!("{what}, marked by {who} on {}", time::local(at, off)),
        _ => what.to_string(),
    };
    match i.excluded.as_deref() {
        Some("planned") => by("Planned work"),
        Some(_) => by("False alarm"),
        None => "Counted in availability".into(),
    }
}

pub fn render(r: &IncidentReport) -> anyhow::Result<Vec<u8>> {
    let i = &r.incident;
    let name = format!("INC-{}", i.id);
    let mut sheet = Sheet::new(
        &format!("{name}: {}", i.title),
        &format!("{}, {name}", r.project),
    );
    sheet.title(&format!("{name}: {}", i.title));
    sheet.note(&format!(
        "This incident belongs to {}, and the report's times are in {}. Monita made it on {}.",
        r.project,
        time::zone(r.off),
        time::stamp(r.now, r.off)
    ));
    let end = i.resolved_at.unwrap_or(r.now);
    sheet.heading("Summary");
    sheet.facts(&[
        (
            "Alert level",
            if i.severity == "critical" {
                "Critical"
            } else {
                "Warning"
            }
            .into(),
        ),
        (
            "Opened by",
            if i.origin == "auto" {
                "Monita, after failed checks"
            } else {
                "A tech lead"
            }
            .into(),
        ),
        ("Started", time::local(i.started_at, r.off)),
        (
            "Resolved",
            i.resolved_at
                .map_or("Still open".into(), |t| time::local(t, r.off)),
        ),
        (
            if i.resolved_at.is_some() {
                "Duration"
            } else {
                "Open for"
            },
            time::duration(end - i.started_at),
        ),
        ("Class", class(i, r.off)),
    ]);

    sheet.heading("Affected systems");
    if r.items.is_empty() {
        sheet.para("This incident names no monitor or heartbeat.");
    }
    for item in &r.items {
        sheet.para(item);
    }

    if let Some(cause) = i.cause.as_deref().filter(|c| !c.trim().is_empty()) {
        sheet.heading("Cause Monita recorded");
        sheet.para(cause);
    }

    sheet.heading("Updates");
    if r.updates.is_empty() {
        sheet.para("Nobody has posted an update yet.");
    } else {
        // Oldest first, so the table reads as a timeline.
        let mut updates: Vec<&UpdateRow> = r.updates.iter().collect();
        updates.sort_by_key(|u| u.created_at);
        let rows: Vec<Vec<String>> = updates
            .iter()
            .map(|u| {
                let mut label = u.label.clone();
                if let Some(c) = label.get_mut(..1) {
                    c.make_ascii_uppercase();
                }
                vec![
                    time::local(u.created_at, r.off),
                    label,
                    author(u),
                    u.message.clone(),
                ]
            })
            .collect();
        sheet.table(
            &[
                Col::left("Time", 0.19),
                Col::left("Label", 0.14),
                Col::left("By", 0.2),
                Col::wrapping("Message", 0.47),
            ],
            &rows,
        );
    }
    sheet.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn update(email: Option<&str>, key: Option<&str>) -> UpdateRow {
        UpdateRow {
            label: "identified".into(),
            message: "m".into(),
            author_email: email.map(str::to_string),
            author_key: key.map(str::to_string),
            created_at: 0,
        }
    }

    #[test]
    fn an_update_names_a_person_a_key_or_monita() {
        assert_eq!(author(&update(Some("rina@x.co"), None)), "rina@x.co");
        assert_eq!(author(&update(None, Some("deploy"))), "API key deploy");
        assert_eq!(author(&update(None, None)), "Monita");
    }
}
