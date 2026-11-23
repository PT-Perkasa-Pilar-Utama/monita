//! Weekly and monthly summary reports by email (v1.2 Q59). Each project has one schedule of each
//! cadence, with a switch and addresses. The daily job sends each closed period once: the first
//! day of the period sent last is recorded in the same write that queues the emails.

use serde::Serialize;

use super::{period, store};
use crate::{
    notify::{self, AlertError, Pdf},
    platform::{caps, db::Db, settings, time},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Cadence {
    Weekly,
    Monthly,
}

impl Cadence {
    pub const ALL: [Cadence; 2] = [Cadence::Weekly, Cadence::Monthly];

    pub fn as_str(self) -> &'static str {
        match self {
            Cadence::Weekly => "weekly",
            Cadence::Monthly => "monthly",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.as_str() == raw)
    }

    /// The period that closed last: Monday to Sunday of last week, or last calendar month.
    pub fn closed(self, now: i64, off: i64) -> period::Range {
        match self {
            Cadence::Weekly => period::last_week(now, off),
            Cadence::Monthly => period::last_month(now, off),
        }
    }
}

/// A schedule as its settings page shows it. A cadence never saved reads off, with no addresses.
#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct Schedule {
    pub cadence: Cadence,
    pub enabled: bool,
    pub emails: Vec<String>,
}

pub async fn list(db: &Db, project_id: i64) -> anyhow::Result<Vec<Schedule>> {
    let saved = store::of_project(&db.read().await?, project_id).await?;
    Ok(Cadence::ALL
        .into_iter()
        .map(|cadence| {
            let row = saved.iter().find(|r| r.cadence == cadence.as_str());
            Schedule {
                cadence,
                enabled: row.is_some_and(|r| r.enabled),
                emails: row.map_or_else(Vec::new, |r| notify::split_emails(&r.emails)),
            }
        })
        .collect())
}

#[derive(Debug, thiserror::Error)]
pub enum ScheduleError {
    #[error(transparent)]
    Emails(#[from] AlertError),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Saves a schedule. A schedule that is on needs its addresses; one that is off keeps what was
/// typed when it is a valid list. Turning it on counts the period that already closed as sent,
/// so the first email goes out when the next period closes.
pub async fn save(
    db: &Db,
    project_id: i64,
    cadence: Cadence,
    enabled: bool,
    emails: &str,
    now: i64,
) -> Result<(), ScheduleError> {
    let list = match notify::email_list(emails) {
        Ok(list) => list,
        Err(AlertError::NoEmails) if !enabled => Vec::new(),
        Err(e) => return Err(e.into()),
    };
    let joined = list.join("\n");
    db.write(async |c| {
        let off = settings::utc_offset(c).await?;
        let before = store::one(c, project_id, cadence.as_str()).await?;
        let was_on = before.as_ref().is_some_and(|r| r.enabled);
        let last_sent = match (&before, enabled && !was_on) {
            (_, true) => Some(time::day_at(cadence.closed(now, off).from, off)),
            (Some(r), false) => r.last_sent.clone(),
            (None, false) => None,
        };
        store::save(
            c,
            project_id,
            cadence.as_str(),
            enabled,
            &joined,
            last_sent.as_deref(),
        )
        .await
    })
    .await?;
    Ok(())
}

/// Sends every schedule whose period closed since it last sent. Returns the emails queued, or
/// the last error after trying every schedule, so the daily job tries the pass again.
pub async fn run_due(db: &Db, base_url: &str, now: i64) -> anyhow::Result<usize> {
    let off = settings::utc_offset(&db.read().await?).await?;
    let mut queued = 0;
    let mut failed = None;
    let mut after = (0_i64, String::new());
    loop {
        let batch =
            store::on_after(&db.read().await?, (after.0, &after.1), caps::JOB_BATCH).await?;
        for row in &batch {
            // One project's failure must not hold back the others; the job retries the pass.
            match send_one(db, base_url, row, now, off).await {
                Ok(n) => queued += n,
                Err(e) => {
                    tracing::error!(project = row.project_id, cadence = %row.cadence, error = %e, "report not sent");
                    failed = Some(e);
                }
            }
        }
        match batch.last() {
            Some(last) if batch.len() as i64 == caps::JOB_BATCH => {
                after = (last.project_id, last.cadence.clone());
            }
            _ => break,
        }
    }
    match failed {
        Some(e) => Err(e),
        None => Ok(queued),
    }
}

/// `YYYY-MM-DD` dates compare in calendar order as text.
fn sent_already(row: &store::Row, first_day: &str) -> bool {
    row.last_sent.as_deref().is_some_and(|d| d >= first_day)
}

async fn send_one(
    db: &Db,
    base_url: &str,
    row: &store::Row,
    now: i64,
    off: i64,
) -> anyhow::Result<usize> {
    let Some(cadence) = Cadence::parse(&row.cadence) else {
        return Ok(0);
    };
    let range = cadence.closed(now, off);
    let first_day = time::day_at(range.from, off);
    if sent_already(row, &first_day) {
        return Ok(0);
    }
    let Some(project) = crate::projects::by_ids(db, &[row.project_id])
        .await?
        .remove(&row.project_id)
    else {
        return Ok(0);
    };
    // The PDF is read before the write, so the writer is never held while it is drawn.
    let pdf = Pdf {
        name: super::file_name(&project.slug, range, off),
        bytes: super::summary_pdf(db, &project, range, now, off).await?,
    };
    let (first, last) = (first_day.clone(), time::day_at(range.to - 1, off));
    let word = cadence.as_str();
    let subject = format!("{} {word} report, {first} to {last}", project.name);
    let body = format!(
        "The {word} report of {} is attached as a PDF. It covers {first} to {last}, with times in {}.\n\n\
         Open the project: {base_url}/projects/{}\n\n\
         You get this email because your address is on the project's {word} report, under \
         Settings, Reports. Ask the project's tech lead to remove it.",
        project.name,
        time::zone(off),
        project.slug,
    );
    db.write(async |c| {
        // Read again inside the write: another pass may have sent it, or a tech lead turned it off
        // or changed the addresses while the PDF was drawn.
        let Some(now_row) = store::one(c, row.project_id, &row.cadence).await? else {
            return Ok(0);
        };
        if !now_row.enabled || sent_already(&now_row, &first_day) {
            return Ok(0);
        }
        store::mark_sent(c, row.project_id, &row.cadence, &first_day).await?;
        let to = notify::split_emails(&now_row.emails);
        for email in &to {
            notify::queue_email_with_pdf(c, email, &subject, &body, &pdf, now).await?;
        }
        Ok(to.len())
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cadence_reads_back_and_names_its_closed_period() {
        for c in Cadence::ALL {
            assert_eq!(Cadence::parse(c.as_str()), Some(c));
        }
        assert_eq!(Cadence::parse("daily"), None);
        let now = 1_795_428_000; // Monday 2026-11-23 10:00 UTC
        let w = Cadence::Weekly.closed(now, 0);
        assert_eq!(time::day_at(w.from, 0), "2026-11-16");
        let m = Cadence::Monthly.closed(now, 0);
        assert_eq!(time::day_at(m.from, 0), "2026-10-01");
    }
}
