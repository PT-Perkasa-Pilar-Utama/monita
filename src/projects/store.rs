//! SQL for projects and api_keys.

use turso::Connection;

use crate::platform::caps;

pub struct Project {
    pub id: i64,
    pub owner_id: i64,
    pub owner_email: String,
    pub owner_active: bool,
    pub name: String,
    pub slug: String,
    pub ip_allowlist: Option<String>,
    pub created_at: i64,
    pub description: Option<String>,
    /// When the icon was last set; `None` when the project has none.
    pub icon_at: Option<i64>,
}

#[derive(serde::Serialize)]
pub struct ApiKeyRow {
    pub id: i64,
    pub name: String,
    pub prefix: String,
    pub last_used_at: Option<i64>,
    pub revoked_at: Option<i64>,
    pub created_at: i64,
}

const COLS: &str = "p.id, p.owner_id, u.email, p.name, p.slug, p.ip_allowlist, p.created_at, u.disabled_at IS NULL, p.description, p.icon_at";

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
        description: r.get(8)?,
        icon_at: r.get(9)?,
    })
}

async fn many(
    c: &Connection,
    sql: String,
    params: impl turso::params::IntoParams,
) -> anyhow::Result<Vec<Project>> {
    crate::platform::db::query_all(c, sql, params, row).await
}

pub async fn insert(
    c: &Connection,
    owner_id: i64,
    name: &str,
    slug: &str,
    description: Option<&str>,
) -> anyhow::Result<i64> {
    c.execute(
        "INSERT INTO projects (owner_id, name, slug, description) VALUES (?1, ?2, ?3, ?4)",
        (owner_id, name, slug, description),
    )
    .await?;
    Ok(c.last_insert_rowid())
}

pub async fn set_description(
    c: &Connection,
    project_id: i64,
    description: Option<&str>,
    at: i64,
) -> anyhow::Result<()> {
    c.execute(
        "UPDATE projects SET description = ?1, updated_at = ?2 WHERE id = ?3",
        (description, at, project_id),
    )
    .await?;
    Ok(())
}

pub async fn set_icon(
    c: &Connection,
    project_id: i64,
    mime: &str,
    data: Vec<u8>,
    at: i64,
) -> anyhow::Result<()> {
    c.execute(
        "INSERT INTO project_icons (project_id, mime, data) VALUES (?1, ?2, ?3)
         ON CONFLICT (project_id) DO UPDATE SET mime = excluded.mime, data = excluded.data",
        (project_id, mime, turso::Value::from(data)),
    )
    .await?;
    c.execute(
        "UPDATE projects SET icon_at = ?1, updated_at = ?1 WHERE id = ?2",
        (at, project_id),
    )
    .await?;
    Ok(())
}

pub async fn remove_icon(c: &Connection, project_id: i64, at: i64) -> anyhow::Result<()> {
    c.execute(
        "DELETE FROM project_icons WHERE project_id = ?1",
        [project_id],
    )
    .await?;
    c.execute(
        "UPDATE projects SET icon_at = NULL, updated_at = ?1 WHERE id = ?2",
        (at, project_id),
    )
    .await?;
    Ok(())
}

/// The icon's type and bytes.
pub async fn icon(c: &Connection, project_id: i64) -> anyhow::Result<Option<(String, Vec<u8>)>> {
    Ok(crate::platform::db::query_all(
        c,
        "SELECT mime, data FROM project_icons WHERE project_id = ?1",
        [project_id],
        |r| {
            let data = r.get_value(1)?.as_blob().cloned().unwrap_or_default();
            Ok((r.get(0)?, data))
        },
    )
    .await?
    .pop())
}

pub async fn name_slug(
    c: &Connection,
    project_id: i64,
) -> anyhow::Result<Option<(String, String)>> {
    Ok(crate::platform::db::query_all(
        c,
        "SELECT name, slug FROM projects WHERE id = ?1",
        [project_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .await?
    .pop())
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

/// Projects one owner has (`?1`, NULL for every project), owned by `?3` (an email, '' for any),
/// whose name, slug, owner email or description holds `?2`. `?2` arrives lower-cased; `lower`
/// folds ASCII only.
const MATCHING: &str = "FROM projects p JOIN users u ON u.id = p.owner_id \
    WHERE (?1 IS NULL OR p.owner_id = ?1) AND (?3 = '' OR u.email = ?3) \
    AND (?2 = '' OR instr(lower(p.name), ?2) > 0 \
    OR instr(lower(p.slug), ?2) > 0 OR instr(lower(u.email), ?2) > 0 \
    OR instr(lower(coalesce(p.description, '')), ?2) > 0)";

/// The project list's sort orders. The id breaks ties so a page boundary never moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sort {
    Name,
    Newest,
    Oldest,
}

impl Sort {
    fn order_by(self) -> &'static str {
        match self {
            Sort::Name => "p.name, p.id",
            Sort::Newest => "p.created_at DESC, p.id DESC",
            Sort::Oldest => "p.created_at, p.id",
        }
    }
}

pub struct Filter<'a> {
    pub owner_id: Option<i64>,
    pub q: &'a str,
    pub owner_email: &'a str,
    pub sort: Sort,
}

pub async fn count_matching(c: &Connection, f: &Filter<'_>) -> anyhow::Result<i64> {
    let sql = format!("SELECT count(*) {MATCHING}");
    Ok(
        crate::platform::db::query_i64(c, &sql, (f.owner_id, f.q, f.owner_email))
            .await?
            .unwrap_or(0),
    )
}

pub async fn page_matching(
    c: &Connection,
    f: &Filter<'_>,
    limit: i64,
    offset: i64,
) -> anyhow::Result<Vec<Project>> {
    let sql = format!(
        "SELECT {COLS} {MATCHING} ORDER BY {} LIMIT ?4 OFFSET ?5",
        f.sort.order_by()
    );
    many(c, sql, (f.owner_id, f.q, f.owner_email, limit, offset)).await
}

/// Who owns at least one project, for the owner filter. Bounded by the account cap.
pub async fn owner_emails(c: &Connection) -> anyhow::Result<Vec<String>> {
    crate::platform::db::query_all(
        c,
        "SELECT DISTINCT u.email FROM projects p JOIN users u ON u.id = p.owner_id ORDER BY u.email LIMIT ?1",
        [caps::USERS],
        |r| r.get(0),
    )
    .await
}

/// A keyset batch in id order, for jobs that walk every project.
pub async fn after_id(c: &Connection, last_id: i64, limit: i64) -> anyhow::Result<Vec<Project>> {
    many(
        c,
        format!("SELECT {COLS} FROM projects p JOIN users u ON u.id = p.owner_id WHERE p.id > ?1 ORDER BY p.id LIMIT ?2"),
        (last_id, limit),
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
    crate::platform::db::query_all(c, "SELECT id, name, key_prefix, last_used_at, revoked_at, created_at FROM api_keys WHERE project_id = ?1 ORDER BY revoked_at IS NOT NULL, coalesce(revoked_at, created_at) DESC LIMIT ?2",
            (project_id, 2 * caps::KEYS_PER_PROJECT), |r| Ok(ApiKeyRow {
            id: r.get(0)?,
            name: r.get(1)?,
            prefix: r.get(2)?,
            last_used_at: r.get(3)?,
            revoked_at: r.get(4)?,
            created_at: r.get(5)?,
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

pub async fn count_live_keys(c: &Connection, project_id: i64) -> anyhow::Result<i64> {
    Ok(crate::platform::db::query_i64(
        c,
        "SELECT count(*) FROM api_keys WHERE project_id = ?1 AND revoked_at IS NULL",
        [project_id],
    )
    .await?
    .unwrap_or(0))
}
