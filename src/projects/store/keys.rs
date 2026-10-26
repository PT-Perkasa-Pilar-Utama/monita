//! SQL for api_keys.

use turso::Connection;

use super::{ApiKeyRow, COLS, Project, row};
use crate::platform::caps;

pub async fn insert_key(
    c: &Connection,
    project_id: i64,
    name: &str,
    hash: &str,
    prefix: &str,
    scope: &str,
) -> anyhow::Result<()> {
    c.execute(
        "INSERT INTO api_keys (project_id, name, key_hash, key_prefix, scope) VALUES (?1, ?2, ?3, ?4, ?5)",
        (project_id, name, hash, prefix, scope),
    )
    .await?;
    Ok(())
}

pub async fn list_keys(c: &Connection, project_id: i64) -> anyhow::Result<Vec<ApiKeyRow>> {
    crate::platform::db::query_all(c, "SELECT id, name, key_prefix, last_used_at, revoked_at, created_at, scope FROM api_keys WHERE project_id = ?1 ORDER BY revoked_at IS NOT NULL, coalesce(revoked_at, created_at) DESC LIMIT ?2",
            (project_id, 2 * caps::KEYS_PER_PROJECT), |r| Ok(ApiKeyRow {
            id: r.get(0)?,
            name: r.get(1)?,
            prefix: r.get(2)?,
            last_used_at: r.get(3)?,
            revoked_at: r.get(4)?,
            created_at: r.get(5)?,
            scope: r.get(6)?,
        })).await
}

pub async fn revoke_key(
    c: &Connection,
    project_id: i64,
    key_id: i64,
    at: i64,
) -> anyhow::Result<()> {
    c.execute(
        "UPDATE api_keys SET revoked_at = ?1 WHERE id = ?2 AND project_id = ?3 AND revoked_at IS NULL",
        (at, key_id, project_id),
    )
    .await?;
    Ok(())
}

/// Returns (key id, project, last_used_at) for a live key.
/// The live key behind `hash`: (key id, its project, when it was last used, its scope).
pub async fn find_live_key(
    c: &Connection,
    hash: &str,
) -> anyhow::Result<Option<(i64, Project, Option<i64>, String)>> {
    let mut rows = c
        .query(
            format!(
                "SELECT {COLS}, k.id, k.last_used_at, k.scope FROM api_keys k
                 JOIN projects p ON p.id = k.project_id JOIN users u ON u.id = p.owner_id
                 WHERE k.key_hash = ?1 AND k.revoked_at IS NULL"
            ),
            [hash],
        )
        .await?;
    // The key's columns follow the project's, however many COLS holds.
    let n = COLS.split(',').count();
    Ok(match rows.next().await? {
        Some(r) => Some((r.get(n)?, row(&r)?, r.get(n + 1)?, r.get(n + 2)?)),
        None => None,
    })
}

pub async fn touch_key(c: &Connection, key_id: i64, at: i64) -> anyhow::Result<()> {
    c.execute(
        "UPDATE api_keys SET last_used_at = ?1 WHERE id = ?2",
        (at, key_id),
    )
    .await?;
    Ok(())
}

pub async fn count_live_keys(c: &Connection, project_id: i64) -> anyhow::Result<i64> {
    Ok(crate::platform::db::query_i64(
        c,
        "SELECT count(*) FROM api_keys WHERE project_id = ?1 AND revoked_at IS NULL",
        [project_id],
    )
    .await?
    .unwrap_or(0))
}
