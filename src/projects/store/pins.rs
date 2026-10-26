//! SQL for `project_pins`.

use crate::platform::{caps, db::query_all, db::query_i64, sql::Connection};

pub async fn count(c: &Connection, user_id: i64) -> anyhow::Result<i64> {
    Ok(query_i64(
        c,
        "SELECT count(*) FROM project_pins WHERE user_id = ?1",
        [user_id],
    )
    .await?
    .unwrap_or(0))
}

pub async fn has(c: &Connection, user_id: i64, project_id: i64) -> anyhow::Result<bool> {
    Ok(query_i64(
        c,
        "SELECT count(*) FROM project_pins WHERE user_id = ?1 AND project_id = ?2",
        [user_id, project_id],
    )
    .await?
    .unwrap_or(0)
        > 0)
}

pub async fn add(c: &Connection, user_id: i64, project_id: i64, at: i64) -> anyhow::Result<()> {
    c.execute(
        "INSERT INTO project_pins (user_id, project_id, pinned_at) VALUES (?1, ?2, ?3) ON CONFLICT DO NOTHING",
        [user_id, project_id, at],
    )
    .await?;
    Ok(())
}

pub async fn remove(c: &Connection, user_id: i64, project_id: i64) -> anyhow::Result<()> {
    c.execute(
        "DELETE FROM project_pins WHERE user_id = ?1 AND project_id = ?2",
        [user_id, project_id],
    )
    .await?;
    Ok(())
}

/// The user's pinned project ids, at most `caps::PINS`.
pub async fn ids(c: &Connection, user_id: i64) -> anyhow::Result<Vec<i64>> {
    query_all(
        c,
        "SELECT project_id FROM project_pins WHERE user_id = ?1 ORDER BY pinned_at, project_id LIMIT ?2",
        [user_id, caps::PINS],
        |r| r.get(0),
    )
    .await
}
