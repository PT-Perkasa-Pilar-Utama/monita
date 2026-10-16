//! SQL for projects and api_keys.

use turso::Connection;

pub struct Project {
    pub id: i64,
    pub owner_id: i64,
    pub owner_email: String,
    pub owner_active: bool,
    pub name: String,
    pub slug: String,
    pub ip_allowlist: Option<String>,
    pub created_at: i64,
}

pub struct ApiKeyRow {
    pub id: i64,
    pub name: String,
    pub prefix: String,
    pub last_used_at: Option<i64>,
    pub revoked_at: Option<i64>,
    pub created_at: i64,
}

const COLS: &str = "p.id, p.owner_id, u.email, p.name, p.slug, p.ip_allowlist, p.created_at, u.disabled_at IS NULL";

fn row(r: &turso::Row) -> turso::Result<Project> {
    Ok(Project {
        id: r.get(0)?,
        owner_id: r.get(1)?,
        owner_email: r.get(2)?,
        name: r.get(3)?,
        slug: r.get(4)?,
        ip_allowlist: r.get(5)?,
        created_at: r.get(6)?,
        owner_active: r.get::<i64>(7)? == 1,
    })
}

async fn many(
    c: &Connection,
    sql: String,
    params: impl turso::params::IntoParams,
) -> anyhow::Result<Vec<Project>> {
    crate::platform::db::query_all(c, sql, params, row).await
}

pub async fn insert(c: &Connection, owner_id: i64, name: &str, slug: &str) -> anyhow::Result<i64> {
    c.execute(
        "INSERT INTO projects (owner_id, name, slug) VALUES (?1, ?2, ?3)",
        (owner_id, name, slug),
    )
    .await?;
    Ok(c.last_insert_rowid())
}

pub async fn find_by_slug(c: &Connection, slug: &str) -> anyhow::Result<Option<Project>> {
    Ok(many(
        c,
        format!(
            "SELECT {COLS} FROM projects p JOIN users u ON u.id = p.owner_id WHERE p.slug = ?1"
        ),
        [slug],
    )
    .await?
    .pop())
}

pub async fn list_all(c: &Connection) -> anyhow::Result<Vec<Project>> {
    many(
        c,
        format!("SELECT {COLS} FROM projects p JOIN users u ON u.id = p.owner_id ORDER BY p.name"),
        (),
    )
    .await
}

pub async fn list_owned(c: &Connection, owner_id: i64) -> anyhow::Result<Vec<Project>> {
    many(
        c,
        format!("SELECT {COLS} FROM projects p JOIN users u ON u.id = p.owner_id WHERE p.owner_id = ?1 ORDER BY p.name"),
        [owner_id],
    )
    .await
}

pub async fn set_owner(
    c: &Connection,
    project_id: i64,
    owner_id: i64,
    at: i64,
) -> anyhow::Result<()> {
    c.execute(
        "UPDATE projects SET owner_id = ?1, updated_at = ?2 WHERE id = ?3",
        (owner_id, at, project_id),
    )
    .await?;
    Ok(())
}

pub async fn set_allowlist(
    c: &Connection,
    project_id: i64,
    list: Option<&str>,
    at: i64,
) -> anyhow::Result<()> {
    c.execute(
        "UPDATE projects SET ip_allowlist = ?1, updated_at = ?2 WHERE id = ?3",
        (list, at, project_id),
    )
    .await?;
    Ok(())
}

pub async fn insert_key(
    c: &Connection,
    project_id: i64,
    name: &str,
    hash: &str,
    prefix: &str,
) -> anyhow::Result<()> {
    c.execute(
        "INSERT INTO api_keys (project_id, name, key_hash, key_prefix) VALUES (?1, ?2, ?3, ?4)",
        (project_id, name, hash, prefix),
    )
    .await?;
    Ok(())
}

pub async fn list_keys(c: &Connection, project_id: i64) -> anyhow::Result<Vec<ApiKeyRow>> {
    let mut rows = c
        .query(
            "SELECT id, name, key_prefix, last_used_at, revoked_at, created_at FROM api_keys WHERE project_id = ?1 ORDER BY revoked_at IS NOT NULL, created_at DESC",
            [project_id],
        )
        .await?;
    let mut out = Vec::new();
    while let Some(r) = rows.next().await? {
        out.push(ApiKeyRow {
            id: r.get(0)?,
            name: r.get(1)?,
            prefix: r.get(2)?,
            last_used_at: r.get(3)?,
            revoked_at: r.get(4)?,
            created_at: r.get(5)?,
        });
    }
    Ok(out)
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
pub async fn find_live_key(
    c: &Connection,
    hash: &str,
) -> anyhow::Result<Option<(i64, Project, Option<i64>)>> {
    let mut rows = c
        .query(
            format!(
                "SELECT {COLS}, k.id, k.last_used_at FROM api_keys k
                 JOIN projects p ON p.id = k.project_id JOIN users u ON u.id = p.owner_id
                 WHERE k.key_hash = ?1 AND k.revoked_at IS NULL"
            ),
            [hash],
        )
        .await?;
    Ok(match rows.next().await? {
        Some(r) => Some((r.get(7)?, row(&r)?, r.get(8)?)),
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
