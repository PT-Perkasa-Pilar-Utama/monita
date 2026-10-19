//! SQL for project_secrets. Only `value_of` reads a value; nothing lists one.

use turso::Connection;

use super::SecretRow;
use crate::platform::{caps, db::query_all};

pub async fn list(c: &Connection, project_id: i64) -> anyhow::Result<Vec<(i64, String, i64)>> {
    query_all(
        c,
        "SELECT id, name, updated_at FROM project_secrets WHERE project_id = ?1 ORDER BY name LIMIT ?2",
        (project_id, caps::SECRETS_PER_PROJECT),
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )
    .await
}

pub async fn count(c: &Connection, project_id: i64) -> anyhow::Result<i64> {
    Ok(crate::platform::db::query_i64(
        c,
        "SELECT count(*) FROM project_secrets WHERE project_id = ?1",
        [project_id],
    )
    .await?
    .unwrap_or(0))
}

pub async fn find(c: &Connection, project_id: i64, id: i64) -> anyhow::Result<Option<SecretRow>> {
    Ok(query_all(
        c,
        "SELECT id, name, updated_at FROM project_secrets WHERE project_id = ?1 AND id = ?2",
        (project_id, id),
        |r| {
            Ok(SecretRow {
                id: r.get(0)?,
                name: r.get(1)?,
                updated_at: r.get(2)?,
                used_by: 0,
            })
        },
    )
    .await?
    .into_iter()
    .next())
}

pub async fn name_taken(c: &Connection, project_id: i64, name: &str) -> anyhow::Result<bool> {
    Ok(crate::platform::db::query_i64(
        c,
        "SELECT count(*) FROM project_secrets WHERE project_id = ?1 AND name = ?2",
        (project_id, name),
    )
    .await?
        == Some(1))
}

pub async fn insert(
    c: &Connection,
    project_id: i64,
    name: &str,
    value: &str,
    at: i64,
) -> anyhow::Result<i64> {
    c.execute(
        "INSERT INTO project_secrets (project_id, name, value, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?4)",
        (project_id, name, value, at),
    )
    .await?;
    Ok(c.last_insert_rowid())
}

pub async fn replace(
    c: &Connection,
    project_id: i64,
    id: i64,
    value: &str,
    at: i64,
) -> anyhow::Result<bool> {
    Ok(c.execute(
        "UPDATE project_secrets SET value = ?1, updated_at = ?2 WHERE project_id = ?3 AND id = ?4",
        (value, at, project_id, id),
    )
    .await?
        > 0)
}

pub async fn delete(c: &Connection, project_id: i64, id: i64) -> anyhow::Result<bool> {
    Ok(c.execute(
        "DELETE FROM project_secrets WHERE project_id = ?1 AND id = ?2",
        (project_id, id),
    )
    .await?
        > 0)
}

pub async fn value_of(c: &Connection, project_id: i64, id: i64) -> anyhow::Result<Option<String>> {
    Ok(query_all(
        c,
        "SELECT value FROM project_secrets WHERE project_id = ?1 AND id = ?2",
        (project_id, id),
        |r| r.get::<String>(0),
    )
    .await?
    .into_iter()
    .next())
}
