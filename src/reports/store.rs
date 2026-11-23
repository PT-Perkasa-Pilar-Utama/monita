//! SQL for `report_schedules`, the only table this module owns (migration 0050).

use crate::platform::sql::Connection;

pub struct Row {
    pub project_id: i64,
    pub cadence: String,
    pub enabled: bool,
    pub emails: String,
    /// The first day of the period sent last, `YYYY-MM-DD` in the admin offset.
    pub last_sent: Option<String>,
}

const COLS: &str = "project_id, cadence, enabled, emails, last_sent";

fn row(r: &crate::platform::sql::Row) -> crate::platform::sql::Result<Row> {
    Ok(Row {
        project_id: r.get(0)?,
        cadence: r.get(1)?,
        enabled: r.get::<i64>(2)? == 1,
        emails: r.get(3)?,
        last_sent: r.get(4)?,
    })
}

/// The project's saved schedules; at most one a cadence.
pub async fn of_project(c: &Connection, project_id: i64) -> anyhow::Result<Vec<Row>> {
    crate::platform::db::query_all(
        c,
        &format!("SELECT {COLS} FROM report_schedules WHERE project_id = ?1 ORDER BY cadence"),
        [project_id],
        row,
    )
    .await
}

pub async fn one(c: &Connection, project_id: i64, cadence: &str) -> anyhow::Result<Option<Row>> {
    Ok(crate::platform::db::query_all(
        c,
        &format!("SELECT {COLS} FROM report_schedules WHERE project_id = ?1 AND cadence = ?2"),
        (project_id, cadence),
        row,
    )
    .await?
    .pop())
}

pub async fn save(
    c: &Connection,
    project_id: i64,
    cadence: &str,
    enabled: bool,
    emails: &str,
    last_sent: Option<&str>,
) -> anyhow::Result<()> {
    c.execute(
        "INSERT INTO report_schedules (project_id, cadence, enabled, emails, last_sent) VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT (project_id, cadence) DO UPDATE SET enabled = excluded.enabled, emails = excluded.emails,
           last_sent = excluded.last_sent",
        (project_id, cadence, i64::from(enabled), emails, last_sent),
    )
    .await?;
    Ok(())
}

/// Schedules that are on, in `(project_id, cadence)` order after the given key, at most `limit`.
pub async fn on_after(c: &Connection, after: (i64, &str), limit: i64) -> anyhow::Result<Vec<Row>> {
    crate::platform::db::query_all(
        c,
        &format!(
            "SELECT {COLS} FROM report_schedules WHERE enabled = 1 AND (project_id, cadence) > (?1, ?2)
             ORDER BY project_id, cadence LIMIT ?3"
        ),
        (after.0, after.1, limit),
        row,
    )
    .await
}

pub async fn mark_sent(
    c: &Connection,
    project_id: i64,
    cadence: &str,
    first_day: &str,
) -> anyhow::Result<()> {
    c.execute(
        "UPDATE report_schedules SET last_sent = ?3 WHERE project_id = ?1 AND cadence = ?2",
        (project_id, cadence, first_day),
    )
    .await?;
    Ok(())
}
