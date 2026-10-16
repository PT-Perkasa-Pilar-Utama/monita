//! SQL for ref_ids and tickets.

use turso::Connection;

use super::NewTicket;

pub struct RefRow {
    pub id: i64,
    pub project_id: i64,
    pub single_use: bool,
    pub used_at: Option<i64>,
    pub revoked_at: Option<i64>,
}

pub struct TicketRow {
    pub number: i64,
    pub title: String,
    pub public_title: Option<String>,
    pub description: String,
    pub page_url: Option<String>,
    pub stage: String,
    pub close_reason: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub ref_revoked: bool,
}

pub struct PublicTicket {
    pub number: i64,
    pub title: String,
    pub stage: String,
    pub close_reason: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(serde::Serialize)]
pub struct MyTicket {
    pub number: i64,
    pub title: String,
    pub stage: String,
    pub close_reason: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

pub async fn insert_ref(
    c: &Connection,
    project_id: i64,
    hash: &str,
    single_use: bool,
    at: i64,
) -> anyhow::Result<()> {
    c.execute(
        "INSERT INTO ref_ids (project_id, token_hash, single_use, created_at) VALUES (?1, ?2, ?3, ?4)",
        (project_id, hash, single_use, at),
    )
    .await?;
    Ok(())
}

pub async fn find_ref(c: &Connection, hash: &str) -> anyhow::Result<Option<RefRow>> {
    let mut rows = c.query("SELECT id, project_id, single_use, used_at, revoked_at FROM ref_ids WHERE token_hash = ?1", [hash]).await?;
    Ok(match rows.next().await? {
        Some(r) => Some(RefRow {
            id: r.get(0)?,
            project_id: r.get(1)?,
            single_use: r.get::<i64>(2)? == 1,
            used_at: r.get(3)?,
            revoked_at: r.get(4)?,
        }),
        None => None,
    })
}

pub async fn use_ref(c: &Connection, id: i64, at: i64) -> anyhow::Result<()> {
    c.execute("UPDATE ref_ids SET used_at = ?1 WHERE id = ?2", (at, id))
        .await?;
    Ok(())
}

pub async fn count_since_for_ref(c: &Connection, ref_id: i64, since: i64) -> anyhow::Result<i64> {
    Ok(crate::platform::db::query_i64(
        c,
        "SELECT count(*) FROM tickets WHERE ref_id = ?1 AND created_at >= ?2",
        (ref_id, since),
    )
    .await?
    .unwrap_or(0))
}

pub async fn count_since_for_project(
    c: &Connection,
    project_id: i64,
    since: i64,
) -> anyhow::Result<i64> {
    Ok(crate::platform::db::query_i64(
        c,
        "SELECT count(*) FROM tickets WHERE project_id = ?1 AND created_at >= ?2",
        (project_id, since),
    )
    .await?
    .unwrap_or(0))
}

/// Returns the new ticket's number. Safe as max + 1 because every write runs on the single
/// writer connection inside one transaction (spec 05 section 5.8.5).
pub async fn insert_ticket(
    c: &Connection,
    project_id: i64,
    ref_id: i64,
    t: &NewTicket,
    at: i64,
) -> anyhow::Result<i64> {
    let number = crate::platform::db::query_i64(
        c,
        "SELECT coalesce(max(number), 0) + 1 FROM tickets WHERE project_id = ?1",
        [project_id],
    )
    .await?
    .unwrap_or(1);
    c.execute(
        "INSERT INTO tickets (project_id, number, ref_id, title, description, page_url, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
        (project_id, number, ref_id, t.title.trim(), t.description.trim(), t.page_url.as_deref(), at),
    )
    .await?;
    Ok(number)
}

const COLS: &str = "t.number, t.title, t.public_title, t.description, t.page_url, t.stage, t.close_reason, t.created_at, t.updated_at, r.revoked_at IS NOT NULL";

fn row(r: &turso::Row) -> turso::Result<TicketRow> {
    Ok(TicketRow {
        number: r.get(0)?,
        title: r.get(1)?,
        public_title: r.get(2)?,
        description: r.get(3)?,
        page_url: r.get(4)?,
        stage: r.get(5)?,
        close_reason: r.get(6)?,
        created_at: r.get(7)?,
        updated_at: r.get(8)?,
        ref_revoked: r.get::<i64>(9)? == 1,
    })
}

pub async fn board(c: &Connection, project_id: i64) -> anyhow::Result<Vec<TicketRow>> {
    let mut rows = c
        .query(format!("SELECT {COLS} FROM tickets t JOIN ref_ids r ON r.id = t.ref_id WHERE t.project_id = ?1 ORDER BY t.updated_at DESC LIMIT 500"), [project_id])
        .await?;
    let mut out = Vec::new();
    while let Some(r) = rows.next().await? {
        out.push(row(&r)?);
    }
    Ok(out)
}

pub async fn find(
    c: &Connection,
    project_id: i64,
    number: i64,
) -> anyhow::Result<Option<TicketRow>> {
    let mut rows = c
        .query(format!("SELECT {COLS} FROM tickets t JOIN ref_ids r ON r.id = t.ref_id WHERE t.project_id = ?1 AND t.number = ?2"), (project_id, number))
        .await?;
    Ok(match rows.next().await? {
        Some(r) => Some(row(&r)?),
        None => None,
    })
}

pub async fn public_board(c: &Connection, project_id: i64) -> anyhow::Result<Vec<PublicTicket>> {
    let mut rows = c
        .query(
            "SELECT number, coalesce(public_title, title), stage, close_reason, created_at, updated_at FROM tickets
             WHERE project_id = ?1 AND stage != 'triage' ORDER BY updated_at DESC LIMIT 500",
            [project_id],
        )
        .await?;
    let mut out = Vec::new();
    while let Some(r) = rows.next().await? {
        out.push(PublicTicket {
            number: r.get(0)?,
            title: r.get(1)?,
            stage: r.get(2)?,
            close_reason: r.get(3)?,
            created_at: r.get(4)?,
            updated_at: r.get(5)?,
        });
    }
    Ok(out)
}

/// The reporter sees their own title, never the description.
pub async fn by_ref(c: &Connection, ref_id: i64) -> anyhow::Result<Vec<MyTicket>> {
    let mut rows = c
        .query("SELECT number, title, stage, close_reason, created_at, updated_at FROM tickets WHERE ref_id = ?1 ORDER BY created_at DESC LIMIT 100", [ref_id])
        .await?;
    let mut out = Vec::new();
    while let Some(r) = rows.next().await? {
        out.push(MyTicket {
            number: r.get(0)?,
            title: r.get(1)?,
            stage: r.get(2)?,
            close_reason: r.get(3)?,
            created_at: r.get(4)?,
            updated_at: r.get(5)?,
        });
    }
    Ok(out)
}

pub async fn set_stage(
    c: &Connection,
    project_id: i64,
    number: i64,
    stage: &str,
    reason: Option<&str>,
    at: i64,
) -> anyhow::Result<bool> {
    let n = c
        .execute(
            "UPDATE tickets SET stage = ?1, close_reason = ?2, updated_at = ?3 WHERE project_id = ?4 AND number = ?5",
            (stage, reason, at, project_id, number),
        )
        .await?;
    Ok(n > 0)
}

pub async fn set_public_title(
    c: &Connection,
    project_id: i64,
    number: i64,
    title: Option<&str>,
    at: i64,
) -> anyhow::Result<bool> {
    let n = c
        .execute("UPDATE tickets SET public_title = ?1, updated_at = ?2 WHERE project_id = ?3 AND number = ?4", (title, at, project_id, number))
        .await?;
    Ok(n > 0)
}

pub async fn revoke_ref_of(
    c: &Connection,
    project_id: i64,
    number: i64,
    at: i64,
) -> anyhow::Result<bool> {
    let n = c
        .execute(
            "UPDATE ref_ids SET revoked_at = coalesce(revoked_at, ?1) WHERE id = (SELECT ref_id FROM tickets WHERE project_id = ?2 AND number = ?3)",
            (at, project_id, number),
        )
        .await?;
    Ok(n > 0)
}

/// Tickets with an id above `after_id` that are still in Triage.
pub async fn new_in_triage_after(
    c: &Connection,
    project_id: i64,
    after_id: i64,
) -> anyhow::Result<Vec<(i64, String)>> {
    let mut rows = c
        .query("SELECT number, title FROM tickets WHERE project_id = ?1 AND stage = 'triage' AND id > ?2 ORDER BY number", (project_id, after_id))
        .await?;
    let mut out = Vec::new();
    while let Some(r) = rows.next().await? {
        out.push((r.get(0)?, r.get(1)?));
    }
    Ok(out)
}

pub async fn max_id(c: &Connection) -> anyhow::Result<i64> {
    Ok(
        crate::platform::db::query_i64(c, "SELECT coalesce(max(id), 0) FROM tickets", ())
            .await?
            .unwrap_or(0),
    )
}
