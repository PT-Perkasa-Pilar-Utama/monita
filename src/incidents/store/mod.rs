//! SQL for incidents, incident_items, incident_sources, status_updates. Writes and single-incident
//! reads live here; `reads` holds the batch and cross-incident reads.

mod reads;

pub use reads::*;

use std::collections::HashMap;

use turso::Connection;

use super::{ItemState, Source, SourceKind, engine::SourceState};
use crate::platform::time;

pub struct IncidentRow {
    pub id: i64,
    pub origin: String,
    pub title: String,
    pub severity: String,
    pub cause: Option<String>,
    pub started_at: i64,
    pub resolved_at: Option<i64>,
    pub last_label: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemRef {
    pub kind: SourceKind,
    pub id: i64,
}

pub struct UpdateRow {
    pub label: String,
    pub message: String,
    pub author_email: Option<String>,
    pub created_at: i64,
}

pub async fn load_source(
    c: &Connection,
    kind: SourceKind,
    id: i64,
) -> anyhow::Result<(SourceState, Option<i64>)> {
    let mut rows = c
        .query(
            "SELECT fail_streak, open_incident_id FROM incident_sources WHERE source_kind = ?1 AND source_id = ?2",
            (kind.as_str(), id),
        )
        .await?;
    Ok(match rows.next().await? {
        Some(r) => {
            let open: Option<i64> = r.get(1)?;
            (
                SourceState {
                    fail_streak: r.get::<i64>(0)? as u32,
                    open: open.is_some(),
                },
                open,
            )
        }
        None => (SourceState::default(), None),
    })
}

pub async fn save_source(
    c: &Connection,
    kind: SourceKind,
    id: i64,
    s: SourceState,
    open: Option<i64>,
) -> anyhow::Result<()> {
    c.execute(
        "INSERT INTO incident_sources (source_kind, source_id, fail_streak, open_incident_id) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT (source_kind, source_id) DO UPDATE SET fail_streak = excluded.fail_streak, open_incident_id = excluded.open_incident_id",
        (kind.as_str(), id, i64::from(s.fail_streak), open),
    )
    .await?;
    Ok(())
}

pub async fn forget_source(c: &Connection, kind: SourceKind, id: i64) -> anyhow::Result<()> {
    c.execute(
        "DELETE FROM incident_sources WHERE source_kind = ?1 AND source_id = ?2",
        (kind.as_str(), id),
    )
    .await?;
    Ok(())
}

pub async fn open_auto(
    c: &Connection,
    s: &Source,
    title: &str,
    cause: &str,
    now: i64,
) -> anyhow::Result<i64> {
    c.execute(
        "INSERT INTO incidents (project_id, origin, title, severity, cause, started_at) VALUES (?1, 'auto', ?2, ?3, ?4, ?5)",
        (s.project_id, title, s.severity.as_str(), cause, now),
    )
    .await?;
    let id = c.last_insert_rowid();
    c.execute(
        "INSERT INTO incident_items (incident_id, source_kind, source_id) VALUES (?1, ?2, ?3)",
        (id, s.kind.as_str(), s.id),
    )
    .await?;
    insert_update(
        c,
        id,
        "investigating",
        &format!("Detected: {cause}."),
        None,
        now,
    )
    .await?;
    Ok(id)
}

/// Closes an automatic incident with `message`, or "Recovered after ..." when `None`. Returns its
/// start time and the severity it opened with.
pub async fn resolve_auto(
    c: &Connection,
    id: i64,
    message: Option<&str>,
    now: i64,
) -> anyhow::Result<(i64, String)> {
    let (started, severity) = {
        let mut rows = c
            .query(
                "SELECT started_at, severity FROM incidents WHERE id = ?1",
                [id],
            )
            .await?;
        match rows.next().await? {
            Some(r) => (r.get::<i64>(0)?, r.get::<String>(1)?),
            None => anyhow::bail!("incident {id} is gone"),
        }
    };
    let recovered = format!("Recovered after {}.", time::duration(now - started));
    resolve_with(c, id, message.unwrap_or(&recovered), now).await?;
    Ok((started, severity))
}

pub async fn resolve_with(c: &Connection, id: i64, message: &str, now: i64) -> anyhow::Result<()> {
    c.execute(
        "UPDATE incidents SET resolved_at = ?1 WHERE id = ?2 AND resolved_at IS NULL",
        (now, id),
    )
    .await?;
    insert_update(c, id, "resolved", message, None, now).await
}

async fn insert_update(
    c: &Connection,
    incident_id: i64,
    label: &str,
    message: &str,
    author: Option<i64>,
    now: i64,
) -> anyhow::Result<()> {
    c.execute(
        "INSERT INTO status_updates (incident_id, label, message, author_id, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        (incident_id, label, message, author, now),
    )
    .await?;
    Ok(())
}

/// (origin, resolved_at) of an incident in the project.
pub async fn origin_and_end(
    c: &Connection,
    project_id: i64,
    incident_id: i64,
) -> anyhow::Result<Option<(String, Option<i64>)>> {
    let mut rows = c
        .query(
            "SELECT origin, resolved_at FROM incidents WHERE id = ?1 AND project_id = ?2",
            (incident_id, project_id),
        )
        .await?;
    Ok(match rows.next().await? {
        Some(r) => Some((r.get(0)?, r.get(1)?)),
        None => None,
    })
}

#[allow(clippy::too_many_arguments)]
pub async fn add_update(
    c: &Connection,
    incident_id: i64,
    origin: &str,
    resolved_at: Option<i64>,
    label: &str,
    message: &str,
    author: Option<i64>,
    now: i64,
) -> anyhow::Result<()> {
    insert_update(c, incident_id, label, message, author, now).await?;
    if label == "resolved" && origin == "manual" && resolved_at.is_none() {
        c.execute(
            "UPDATE incidents SET resolved_at = ?1 WHERE id = ?2",
            (now, incident_id),
        )
        .await?;
    }
    Ok(())
}

pub async fn open_manual(
    c: &Connection,
    n: &super::NewIncident,
    items: &[ItemRef],
    now: i64,
) -> anyhow::Result<i64> {
    c.execute(
        "INSERT INTO incidents (project_id, origin, title, severity, started_at, created_by) VALUES (?1, 'manual', ?2, ?3, ?4, ?5)",
        (n.project_id, n.title.trim(), n.severity.as_str(), now, n.author),
    )
    .await?;
    let id = c.last_insert_rowid();
    for item in items {
        c.execute(
            "INSERT OR IGNORE INTO incident_items (incident_id, source_kind, source_id) VALUES (?1, ?2, ?3)",
            (id, item.kind.as_str(), item.id),
        )
        .await?;
    }
    insert_update(
        c,
        id,
        "investigating",
        n.message.trim(),
        Some(n.author),
        now,
    )
    .await?;
    Ok(id)
}

const COLS: &str = "i.id, i.origin, i.title, i.severity, i.cause, i.started_at, i.resolved_at,
    (SELECT label FROM status_updates WHERE incident_id = i.id ORDER BY created_at DESC, id DESC LIMIT 1)";

fn row(r: &turso::Row) -> turso::Result<IncidentRow> {
    Ok(IncidentRow {
        id: r.get(0)?,
        origin: r.get(1)?,
        title: r.get(2)?,
        severity: r.get(3)?,
        cause: r.get(4)?,
        started_at: r.get(5)?,
        resolved_at: r.get(6)?,
        last_label: r.get(7)?,
    })
}

pub async fn list(c: &Connection, project_id: i64, open: bool) -> anyhow::Result<Vec<IncidentRow>> {
    let filter = if open {
        "i.resolved_at IS NULL"
    } else {
        "i.resolved_at IS NOT NULL"
    };
    crate::platform::db::query_all(c, format!("SELECT {COLS} FROM incidents i WHERE i.project_id = ?1 AND {filter} ORDER BY i.started_at DESC LIMIT 200"), [project_id], row).await
}

pub async fn find(c: &Connection, project_id: i64, id: i64) -> anyhow::Result<Option<IncidentRow>> {
    let mut rows = c
        .query(
            format!("SELECT {COLS} FROM incidents i WHERE i.project_id = ?1 AND i.id = ?2"),
            (project_id, id),
        )
        .await?;
    Ok(match rows.next().await? {
        Some(r) => Some(row(&r)?),
        None => None,
    })
}
