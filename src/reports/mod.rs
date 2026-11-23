//! Project reports as PDF (v1.2 Q56 to Q61): a summary of a range of days, the report of one
//! incident, and the weekly and monthly emails. Owns `report_schedules`; it reads every number
//! through the other modules' interfaces.

pub mod api;
mod incident;
mod period;
pub mod postmortem;
mod schedule;
mod sheet;
mod store;
mod summary;

pub use period::{Range, RangeError, last_7_days, last_month, last_quarter, last_week};
pub use schedule::{
    Cadence, Schedule, ScheduleError, list as schedules, run_due, save as save_schedule,
};

use crate::{platform::db::Db, projects::Project};

/// The summary report of `range`, as a PDF.
pub async fn summary_pdf(
    db: &Db,
    project: &Project,
    range: Range,
    now: i64,
    off: i64,
) -> anyhow::Result<Vec<u8>> {
    summary::render(&summary::gather(db, project, range, now, off).await?)
}

/// The report of the project's incident `id`, with its post-mortem as stored, as a PDF; `None`
/// when the project has no such incident.
pub async fn incident_pdf(
    db: &Db,
    project: &Project,
    id: i64,
    now: i64,
    off: i64,
) -> anyhow::Result<Option<Vec<u8>>> {
    let pm = postmortem::for_report(db, project.id, id).await?;
    incident_pdf_with(db, project, id, pm.as_ref().map(|(t, d)| (t, *d)), now, off).await
}

/// The incident report with the given post-mortem text, `true` when it is a draft. A publish
/// passes the text it is about to store.
pub async fn incident_pdf_with(
    db: &Db,
    project: &Project,
    id: i64,
    postmortem: Option<(&postmortem::Text, bool)>,
    now: i64,
    off: i64,
) -> anyhow::Result<Option<Vec<u8>>> {
    match incident::gather(db, project, id, now, off).await? {
        Some(r) => Ok(Some(incident::render(&r, postmortem)?)),
        None => Ok(None),
    }
}

/// A range of days from `?period=` or `?from=&to=`.
pub fn range_of(
    period: &str,
    from: &str,
    to: &str,
    now: i64,
    off: i64,
) -> Result<Range, RangeError> {
    match period {
        "week" => Ok(last_week(now, off)),
        "7d" => Ok(last_7_days(now, off)),
        "month" => Ok(last_month(now, off)),
        "quarter" => Ok(last_quarter(now, off)),
        _ => period::days(from, to, now, off),
    }
}

/// The download's file name: `payments-2026-10-01-to-2026-10-31.pdf`.
pub fn file_name(slug: &str, range: Range, off: i64) -> String {
    use crate::platform::time::day_at;
    format!(
        "{slug}-{}-to-{}.pdf",
        day_at(range.from, off),
        day_at(range.to - 1, off)
    )
}
