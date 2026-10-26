//! SQL for projects. API keys are in `keys`.

mod keys;

pub use keys::*;

use turso::Connection;

use super::PublicPages;
use crate::platform::caps;

pub struct Project {
    pub id: i64,
    pub owner_id: i64,
    pub owner_email: String,
    pub owner_active: bool,
    /// When the owner's picture last changed; `None` without one. Read with the owner's email.
    pub owner_avatar_at: Option<i64>,
    pub name: String,
    pub slug: String,
    pub ip_allowlist: Option<String>,
    pub created_at: i64,
    pub description: Option<String>,
    /// When the icon was last set; `None` when the project has none.
    pub icon_at: Option<i64>,
    pub pages: PublicPages,
    /// Where people use the system; `None` for a project made before it was asked.
    pub homepage_url: Option<String>,
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

const COLS: &str = "p.id, p.owner_id, u.email, p.name, p.slug, p.ip_allowlist, p.created_at, u.disabled_at IS NULL, p.description, p.icon_at, p.show_known_issues, p.show_report, p.show_incidents, p.homepage_url, p.show_changelog, u.avatar_at";

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
        pages: PublicPages {
            incidents: r.get::<i64>(12)? == 1,
            known_issues: r.get::<i64>(10)? == 1,
            report: r.get::<i64>(11)? == 1,
            changelog: r.get::<i64>(14)? == 1,
        },
        homepage_url: r.get(13)?,
        owner_avatar_at: r.get(15)?,
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
    homepage_url: &str,
) -> anyhow::Result<i64> {
    c.execute(
        // The column defaults to 0 so that migration 0021 left old status pages alone; a new
        // project starts with its Changelog tab on (v1.1 Q28).
        "INSERT INTO projects (owner_id, name, slug, description, homepage_url, show_changelog) VALUES (?1, ?2, ?3, ?4, ?5, 1)",
        (owner_id, name, slug, description, homepage_url),
    )
    .await?;
    Ok(c.last_insert_rowid())
}

pub async fn set_description(
    c: &Connection,
    project_id: i64,
    description: Option<&str>,
    homepage_url: &str,
    at: i64,
) -> anyhow::Result<()> {
    c.execute(
        "UPDATE projects SET description = ?1, homepage_url = ?4, updated_at = ?2 WHERE id = ?3",
        (description, at, project_id, homepage_url),
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

pub async fn owner_email(c: &Connection, project_id: i64) -> anyhow::Result<Option<String>> {
    Ok(crate::platform::db::query_all(
        c,
        "SELECT u.email FROM projects p JOIN users u ON u.id = p.owner_id
         WHERE p.id = ?1 AND u.disabled_at IS NULL",
        [project_id],
        |r| r.get::<String>(0),
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
    /// The state filter as project ids: only these, or all but these. `None` keeps every state.
    pub ids: Option<Ids>,
}

/// Project ids the state filter keeps or drops.
pub enum Ids {
    Only(Vec<i64>),
    Not(Vec<i64>),
}

impl Filter<'_> {
    /// The state filter's clause. Only plain integers reach the SQL.
    fn state_sql(&self) -> String {
        let list = |ids: &[i64]| ids.iter().map(i64::to_string).collect::<Vec<_>>().join(",");
        match &self.ids {
            None => String::new(),
            Some(Ids::Only(ids)) if ids.is_empty() => " AND 0".into(),
            Some(Ids::Only(ids)) => format!(" AND p.id IN ({})", list(ids)),
            Some(Ids::Not(ids)) if ids.is_empty() => String::new(),
            Some(Ids::Not(ids)) => format!(" AND p.id NOT IN ({})", list(ids)),
        }
    }
}

pub async fn count_matching(c: &Connection, f: &Filter<'_>) -> anyhow::Result<i64> {
    let sql = format!("SELECT count(*) {MATCHING}{}", f.state_sql());
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
        "SELECT {COLS} {MATCHING}{} ORDER BY {} LIMIT ?4 OFFSET ?5",
        f.state_sql(),
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

pub async fn set_pages(
    c: &Connection,
    project_id: i64,
    p: PublicPages,
    at: i64,
) -> anyhow::Result<()> {
    c.execute(
        "UPDATE projects SET show_known_issues = ?1, show_report = ?2, show_incidents = ?5, show_changelog = ?6, updated_at = ?3 WHERE id = ?4",
        (p.known_issues as i64, p.report as i64, at, project_id, p.incidents as i64, p.changelog as i64),
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

/// Deletes the project. Its keys, routes, icon, monitors, heartbeats, incidents, ref IDs and
/// tickets cascade with it.
// ponytail: one cascading DELETE; a project with months of check results holds the writer for
// that long. Delete check_results in batches first if that proves too slow.
pub async fn delete_project(c: &Connection, id: i64) -> anyhow::Result<()> {
    c.execute("DELETE FROM projects WHERE id = ?1", [id])
        .await?;
    Ok(())
}

/// The projects with these ids, at most one page of a cross-project list.
pub async fn by_ids(c: &Connection, ids: &[i64]) -> anyhow::Result<Vec<Project>> {
    let list = ids.iter().map(i64::to_string).collect::<Vec<_>>().join(",");
    many(
        c,
        format!("SELECT {COLS} FROM projects p JOIN users u ON u.id = p.owner_id WHERE p.id IN ({list}) LIMIT ?1"),
        [caps::ACROSS_PAGE],
    )
    .await
}

/// Whether the user owns any project, active or not.
pub async fn owns_any(c: &Connection, user_id: i64) -> anyhow::Result<bool> {
    Ok(crate::platform::db::query_i64(
        c,
        "SELECT EXISTS (SELECT 1 FROM projects WHERE owner_id = ?1)",
        [user_id],
    )
    .await?
    .unwrap_or(0)
        == 1)
}
