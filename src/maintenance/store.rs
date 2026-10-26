//! SQL for maintenance_windows and maintenance_items. A window is active while
//! `starts_at <= now < ends_at`.

use std::collections::HashMap;

use crate::platform::sql::Connection;

use crate::{
    incidents::{ItemRef, SourceKind},
    platform::{
        caps,
        db::{query_all, query_i64},
    },
};

#[derive(Clone, Debug)]
pub struct Window {
    pub id: i64,
    pub title: String,
    pub message: String,
    pub starts_at: i64,
    pub ends_at: i64,
    pub items: Vec<ItemRef>,
}

const COLS: &str = "id, title, message, starts_at, ends_at";

fn window(r: &crate::platform::sql::Row) -> crate::platform::sql::Result<Window> {
    Ok(Window {
        id: r.get(0)?,
        title: r.get(1)?,
        message: r.get(2)?,
        starts_at: r.get(3)?,
        ends_at: r.get(4)?,
        items: Vec::new(),
    })
}

#[allow(clippy::too_many_arguments)]
pub async fn insert(
    c: &Connection,
    project_id: i64,
    title: &str,
    message: &str,
    starts_at: i64,
    ends_at: i64,
    items: &[ItemRef],
    now: i64,
) -> anyhow::Result<i64> {
    c.execute(
        "INSERT INTO maintenance_windows (project_id, title, message, starts_at, ends_at, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        (project_id, title, message, starts_at, ends_at, now),
    )
    .await?;
    let id = c.last_insert_rowid();
    for item in items {
        c.execute(
            "INSERT INTO maintenance_items (window_id, source_kind, source_id) VALUES (?1, ?2, ?3)",
            (id, item.kind.as_str(), item.id),
        )
        .await?;
    }
    Ok(id)
}

/// Windows not yet ended: what the per-project cap counts.
pub async fn open_count(c: &Connection, project_id: i64, now: i64) -> anyhow::Result<i64> {
    Ok(query_i64(
        c,
        "SELECT count(*) FROM maintenance_windows WHERE project_id = ?1 AND ends_at > ?2",
        (project_id, now),
    )
    .await?
    .unwrap_or(0))
}

/// (starts_at, ends_at) of the project's window `id`.
pub async fn times(c: &Connection, project_id: i64, id: i64) -> anyhow::Result<Option<(i64, i64)>> {
    Ok(query_all(
        c,
        "SELECT starts_at, ends_at FROM maintenance_windows WHERE id = ?1 AND project_id = ?2",
        (id, project_id),
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .await?
    .into_iter()
    .next())
}

pub async fn delete(c: &Connection, id: i64) -> anyhow::Result<()> {
    c.execute("DELETE FROM maintenance_windows WHERE id = ?1", (id,))
        .await?;
    Ok(())
}

pub async fn set_end(c: &Connection, id: i64, at: i64) -> anyhow::Result<()> {
    c.execute(
        "UPDATE maintenance_windows SET ends_at = ?2 WHERE id = ?1",
        (id, at),
    )
    .await?;
    Ok(())
}

pub async fn count(
    c: &Connection,
    project_id: i64,
    q: &str,
    state: &str,
    now: i64,
) -> anyhow::Result<i64> {
    Ok(query_i64(
        c,
        &format!("SELECT count(*) FROM maintenance_windows WHERE {MATCHES}"),
        (project_id, search(q), state, now),
    )
    .await?
    .unwrap_or(0))
}

/// Fills each window's items. A window covers at most every source of its project.
async fn with_items(c: &Connection, mut windows: Vec<Window>) -> anyhow::Result<Vec<Window>> {
    if windows.is_empty() {
        return Ok(windows);
    }
    let ids: Vec<String> = windows.iter().map(|w| w.id.to_string()).collect();
    let rows = query_all(
        c,
        format!(
            "SELECT window_id, source_kind, source_id FROM maintenance_items WHERE window_id IN ({}) LIMIT ?1",
            ids.join(",")
        ),
        (windows.len() as i64 * caps::SOURCES_PER_PROJECT,),
        |r| Ok((r.get::<i64>(0)?, r.get::<String>(1)?, r.get::<i64>(2)?)),
    )
    .await?;
    let mut by_window: HashMap<i64, Vec<ItemRef>> = HashMap::new();
    for (window_id, kind, id) in rows {
        if let Some(kind) = SourceKind::parse(&kind) {
            by_window
                .entry(window_id)
                .or_default()
                .push(ItemRef { kind, id });
        }
    }
    for w in &mut windows {
        w.items = by_window.remove(&w.id).unwrap_or_default();
    }
    Ok(windows)
}

/// The windows a page lists: `?2` a title search, `?3` a state (`''` for all), `?4` the time
/// that decides the state, the same rule as `maintenance::state`.
const MATCHES: &str = "project_id = ?1 AND (?2 = '' OR instr(lower(title), ?2) > 0) \
    AND (?3 = '' OR (?3 = 'scheduled' AND starts_at > ?4) \
         OR (?3 = 'active' AND starts_at <= ?4 AND ends_at > ?4) \
         OR (?3 = 'ended' AND ends_at <= ?4))";

fn search(q: &str) -> String {
    q.trim().to_lowercase().chars().take(100).collect()
}

/// Latest start first, or the earliest with `oldest`.
#[allow(clippy::too_many_arguments)]
pub async fn page(
    c: &Connection,
    project_id: i64,
    q: &str,
    state: &str,
    now: i64,
    oldest: bool,
    limit: i64,
    offset: i64,
) -> anyhow::Result<Vec<Window>> {
    let order = if oldest {
        "starts_at ASC, id ASC"
    } else {
        "starts_at DESC, id DESC"
    };
    let windows = query_all(
        c,
        format!("SELECT {COLS} FROM maintenance_windows WHERE {MATCHES} ORDER BY {order} LIMIT ?5 OFFSET ?6"),
        (project_id, search(q), state, now, limit, offset),
        window,
    )
    .await?;
    with_items(c, windows).await
}

/// Active windows, soonest end first. Active windows are open windows, so the cap bounds them.
pub async fn active(c: &Connection, project_id: i64, at: i64) -> anyhow::Result<Vec<Window>> {
    let windows = query_all(
        c,
        format!("SELECT {COLS} FROM maintenance_windows WHERE project_id = ?1 AND starts_at <= ?2 AND ends_at > ?2 ORDER BY ends_at, id LIMIT ?3"),
        (project_id, at, caps::OPEN_WINDOWS_PER_PROJECT),
        window,
    )
    .await?;
    with_items(c, windows).await
}

pub async fn covers(
    c: &Connection,
    kind: SourceKind,
    id: i64,
    at: i64,
) -> anyhow::Result<Option<i64>> {
    query_i64(
        c,
        "SELECT max(w.ends_at) FROM maintenance_items i JOIN maintenance_windows w ON w.id = i.window_id
         WHERE i.source_kind = ?1 AND i.source_id = ?2 AND w.starts_at <= ?3 AND w.ends_at > ?3",
        (kind.as_str(), id, at),
    )
    .await
}

/// (starts_at, ends_at, title) of the windows over one source that overlap `[from, to)`, oldest
/// first, at most `caps::SPANS`.
pub async fn spans(
    c: &Connection,
    kind: SourceKind,
    id: i64,
    from: i64,
    to: i64,
) -> anyhow::Result<Vec<(i64, i64, String)>> {
    let mut rows = c
        .query(
            "SELECT w.starts_at, w.ends_at, w.title FROM maintenance_items i JOIN maintenance_windows w ON w.id = i.window_id
             WHERE i.source_kind = ?1 AND i.source_id = ?2 AND w.starts_at < ?4 AND w.ends_at > ?3
             ORDER BY w.starts_at LIMIT ?5",
            (kind.as_str(), id, from, to, crate::platform::caps::SPANS),
        )
        .await?;
    let mut out = Vec::new();
    while let Some(r) = rows.next().await? {
        out.push((r.get(0)?, r.get(1)?, r.get(2)?));
    }
    Ok(out)
}
