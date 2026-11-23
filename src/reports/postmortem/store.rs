//! SQL for `postmortems` and `postmortem_recipients` (migration 0051), owned by `reports`.

use super::{State, Text};
use crate::platform::sql::Connection;

pub struct Row {
    pub state: State,
    pub text: Text,
    pub updated_by: String,
    pub updated_at: i64,
    pub published_at: Option<i64>,
}

const COLS: &str = "state, summary, impact, root_cause, trigger_cause, resolution, detection, action_items, lessons, updated_by, updated_at, published_at";

pub async fn one(c: &Connection, project_id: i64, incident_id: i64) -> anyhow::Result<Option<Row>> {
    Ok(crate::platform::db::query_all(
        c,
        &format!("SELECT {COLS} FROM postmortems WHERE project_id = ?1 AND incident_id = ?2"),
        (project_id, incident_id),
        |r| {
            let state: String = r.get(0)?;
            Ok(Row {
                state: if state == "published" {
                    State::Published
                } else {
                    State::Draft
                },
                text: Text {
                    summary: r.get(1)?,
                    impact: r.get(2)?,
                    root_cause: r.get(3)?,
                    trigger: r.get(4)?,
                    resolution: r.get(5)?,
                    detection: r.get(6)?,
                    action_items: r.get(7)?,
                    lessons: r.get(8)?,
                },
                updated_by: r.get(9)?,
                updated_at: r.get(10)?,
                published_at: r.get(11)?,
            })
        },
    )
    .await?
    .pop())
}

/// Stores the text in `state`. `published_at` keeps the first publish.
pub async fn save(
    c: &Connection,
    project_id: i64,
    incident_id: i64,
    state: State,
    t: &Text,
    by: &str,
    now: i64,
) -> anyhow::Result<()> {
    let published = (state == State::Published).then_some(now);
    c.execute(
        "INSERT INTO postmortems (incident_id, project_id, state, summary, impact, root_cause, trigger_cause, resolution,
           detection, action_items, lessons, updated_by, updated_at, published_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
         ON CONFLICT (incident_id) DO UPDATE SET state = excluded.state, summary = excluded.summary,
           impact = excluded.impact, root_cause = excluded.root_cause, trigger_cause = excluded.trigger_cause,
           resolution = excluded.resolution, detection = excluded.detection, action_items = excluded.action_items,
           lessons = excluded.lessons, updated_by = excluded.updated_by, updated_at = excluded.updated_at,
           published_at = coalesce(postmortems.published_at, excluded.published_at)",
        (
            incident_id,
            project_id,
            state.as_str(),
            t.summary.as_str(),
            t.impact.as_str(),
            t.root_cause.as_str(),
            t.trigger.as_str(),
            t.resolution.as_str(),
            t.detection.as_str(),
            t.action_items.as_str(),
            t.lessons.as_str(),
            by,
            now,
            published,
        ),
    )
    .await?;
    Ok(())
}

pub async fn set_state(
    c: &Connection,
    incident_id: i64,
    state: State,
    by: &str,
    now: i64,
) -> anyhow::Result<()> {
    c.execute(
        "UPDATE postmortems SET state = ?2, updated_by = ?3, updated_at = ?4 WHERE incident_id = ?1",
        (incident_id, state.as_str(), by, now),
    )
    .await?;
    Ok(())
}

pub async fn delete(c: &Connection, incident_id: i64) -> anyhow::Result<()> {
    c.execute(
        "DELETE FROM postmortems WHERE incident_id = ?1",
        [incident_id],
    )
    .await?;
    Ok(())
}

/// The post-mortem list of a project: whether it is on, and its addresses one a line.
pub async fn recipients(c: &Connection, project_id: i64) -> anyhow::Result<(bool, String)> {
    Ok(crate::platform::db::query_all(
        c,
        "SELECT enabled, emails FROM postmortem_recipients WHERE project_id = ?1",
        [project_id],
        |r| Ok((r.get::<i64>(0)? == 1, r.get(1)?)),
    )
    .await?
    .pop()
    .unwrap_or((false, String::new())))
}

pub async fn save_recipients(
    c: &Connection,
    project_id: i64,
    enabled: bool,
    emails: &str,
) -> anyhow::Result<()> {
    c.execute(
        "INSERT INTO postmortem_recipients (project_id, enabled, emails) VALUES (?1, ?2, ?3)
         ON CONFLICT (project_id) DO UPDATE SET enabled = excluded.enabled, emails = excluded.emails",
        (project_id, i64::from(enabled), emails),
    )
    .await?;
    Ok(())
}
