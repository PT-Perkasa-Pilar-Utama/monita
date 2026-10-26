//! SQL for `monitor_pins` and `heartbeat_pins`.

use crate::{
    incidents::SourceKind,
    platform::{caps, db::query_all, db::query_i64, sql::Connection},
};

/// (pin table, its id column, the source table) for a kind.
fn tables(kind: SourceKind) -> (&'static str, &'static str, &'static str) {
    match kind {
        SourceKind::Monitor => ("monitor_pins", "monitor_id", "monitors"),
        SourceKind::Heartbeat => ("heartbeat_pins", "heartbeat_id", "heartbeats"),
    }
}

/// The user's pins of this kind in the project.
pub async fn count(
    c: &Connection,
    user_id: i64,
    project_id: i64,
    kind: SourceKind,
) -> anyhow::Result<i64> {
    let (pins, col, src) = tables(kind);
    Ok(query_i64(
        c,
        &format!("SELECT count(*) FROM {pins} p JOIN {src} s ON s.id = p.{col} WHERE p.user_id = ?1 AND s.project_id = ?2"),
        [user_id, project_id],
    )
    .await?
    .unwrap_or(0))
}

pub async fn has(c: &Connection, user_id: i64, kind: SourceKind, id: i64) -> anyhow::Result<bool> {
    let (pins, col, _) = tables(kind);
    Ok(query_i64(
        c,
        &format!("SELECT count(*) FROM {pins} WHERE user_id = ?1 AND {col} = ?2"),
        [user_id, id],
    )
    .await?
    .unwrap_or(0)
        > 0)
}

pub async fn add(
    c: &Connection,
    user_id: i64,
    kind: SourceKind,
    id: i64,
    at: i64,
) -> anyhow::Result<()> {
    let (pins, col, _) = tables(kind);
    c.execute(
        format!("INSERT INTO {pins} (user_id, {col}, pinned_at) VALUES (?1, ?2, ?3) ON CONFLICT DO NOTHING"),
        [user_id, id, at],
    )
    .await?;
    Ok(())
}

pub async fn remove(c: &Connection, user_id: i64, kind: SourceKind, id: i64) -> anyhow::Result<()> {
    let (pins, col, _) = tables(kind);
    c.execute(
        format!("DELETE FROM {pins} WHERE user_id = ?1 AND {col} = ?2"),
        [user_id, id],
    )
    .await?;
    Ok(())
}

/// The user's pinned ids of this kind in the project, at most `caps::PINS`.
pub async fn ids(
    c: &Connection,
    user_id: i64,
    project_id: i64,
    kind: SourceKind,
) -> anyhow::Result<Vec<i64>> {
    let (pins, col, src) = tables(kind);
    query_all(
        c,
        format!("SELECT p.{col} FROM {pins} p JOIN {src} s ON s.id = p.{col} WHERE p.user_id = ?1 AND s.project_id = ?2 ORDER BY p.pinned_at, p.{col} LIMIT ?3"),
        [user_id, project_id, caps::PINS],
        |r| r.get(0),
    )
    .await
}
