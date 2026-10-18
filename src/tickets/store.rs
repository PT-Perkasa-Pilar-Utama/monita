//! SQL for ref_ids and tickets.

use turso::Connection;

use crate::platform::caps;

use super::NewTicket;

pub struct RefRow {
    pub id: i64,
    pub project_id: i64,
    pub single_use: bool,
    pub used_at: Option<i64>,
    pub revoked_at: Option<i64>,
}

#[derive(serde::Serialize)]
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
    /// The team member who filed it; `None` for a reporter's ticket.
    pub filed_by: Option<String>,
}

/// SEC-14 public fields only: number, title, stage, dates. The close reason is the TL's verdict
/// and stays private.
#[derive(serde::Serialize)]
pub struct PublicTicket {
    pub number: i64,
    pub title: String,
    pub stage: String,
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

/// Who files a ticket: a reporter through a ref ID, or a team member.
pub enum Filer {
    Ref(i64),
    Member(i64),
}

/// Returns the new ticket's number. Safe as max + 1 because every write runs on the single
/// writer connection inside one transaction (spec 05 section 5.8.5).
pub async fn insert_ticket(
    c: &Connection,
    project_id: i64,
    by: Filer,
    stage: &str,
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
    let (ref_id, created_by) = match by {
        Filer::Ref(id) => (Some(id), None),
        Filer::Member(id) => (None, Some(id)),
    };
    c.execute(
        "INSERT INTO tickets (project_id, number, ref_id, created_by, stage, title, description, page_url, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
        (project_id, number, ref_id, created_by, stage, t.title.trim(), t.description.trim(), t.page_url.as_deref(), at),
    )
    .await?;
    Ok(number)
}

const COLS: &str = "t.number, t.title, t.public_title, t.description, t.page_url, t.stage, t.close_reason, t.created_at, t.updated_at, r.revoked_at IS NOT NULL, u.email";
/// A team ticket has no ref ID, and a reporter's ticket no member, so both joins are LEFT.
const WHO: &str = "LEFT JOIN ref_ids r ON r.id = t.ref_id LEFT JOIN users u ON u.id = t.created_by";

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
        filed_by: r.get(10)?,
    })
}

/// The newest `caps::TICKETS_PER_COLUMN` tickets of each stage, as `t`: a board stays readable
/// after years of tickets. `stage_counts` gives the column totals.
const NEWEST_PER_STAGE: &str = "(SELECT *, row_number() OVER (PARTITION BY stage ORDER BY updated_at DESC, id DESC) AS rn FROM tickets WHERE project_id = ?1) t";

pub async fn board(c: &Connection, project_id: i64) -> anyhow::Result<Vec<TicketRow>> {
    crate::platform::db::query_all(c, format!("SELECT {COLS} FROM {NEWEST_PER_STAGE} {WHO} WHERE t.rn <= ?2 ORDER BY t.updated_at DESC"), (project_id, caps::TICKETS_PER_COLUMN), row).await
}

fn public_row(r: &turso::Row) -> turso::Result<PublicTicket> {
    Ok(PublicTicket {
        number: r.get(0)?,
        title: r.get(1)?,
        stage: r.get(2)?,
        created_at: r.get(3)?,
        updated_at: r.get(4)?,
    })
}

/// One page of one column, in the board's order, for Show more and the Done list.
pub async fn column_page(
    c: &Connection,
    project_id: i64,
    stage: &str,
    offset: i64,
) -> anyhow::Result<Vec<TicketRow>> {
    crate::platform::db::query_all(c, format!("SELECT {COLS} FROM tickets t {WHO} WHERE t.project_id = ?1 AND t.stage = ?2 ORDER BY t.updated_at DESC, t.id DESC LIMIT ?3 OFFSET ?4"), (project_id, stage, caps::TICKETS_PER_COLUMN, offset), row).await
}

/// The same page with public fields only (SEC-14). The caller never asks for Triage.
pub async fn public_column_page(
    c: &Connection,
    project_id: i64,
    stage: &str,
    offset: i64,
) -> anyhow::Result<Vec<PublicTicket>> {
    crate::platform::db::query_all(c, "SELECT number, coalesce(public_title, title), stage, created_at, updated_at FROM tickets WHERE project_id = ?1 AND stage = ?2 AND stage != 'triage' ORDER BY updated_at DESC, id DESC LIMIT ?3 OFFSET ?4", (project_id, stage, caps::TICKETS_PER_COLUMN, offset), public_row).await
}

/// (stage, tickets in it) for the stages that have any.
pub async fn stage_counts(c: &Connection, project_id: i64) -> anyhow::Result<Vec<(String, i64)>> {
    crate::platform::db::query_all(
        c,
        "SELECT stage, count(*) FROM tickets WHERE project_id = ?1 GROUP BY stage",
        [project_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .await
}

pub async fn find(
    c: &Connection,
    project_id: i64,
    number: i64,
) -> anyhow::Result<Option<TicketRow>> {
    let mut rows = c
        .query(
            format!("SELECT {COLS} FROM tickets t {WHO} WHERE t.project_id = ?1 AND t.number = ?2"),
            (project_id, number),
        )
        .await?;
    Ok(match rows.next().await? {
        Some(r) => Some(row(&r)?),
        None => None,
    })
}

pub async fn public_board(c: &Connection, project_id: i64) -> anyhow::Result<Vec<PublicTicket>> {
    crate::platform::db::query_all(
        c,
        format!("SELECT t.number, coalesce(t.public_title, t.title), t.stage, t.created_at, t.updated_at FROM {NEWEST_PER_STAGE}
         WHERE t.stage != 'triage' AND t.rn <= ?2 ORDER BY t.updated_at DESC"),
        (project_id, caps::TICKETS_PER_COLUMN),
        public_row,
    )
    .await
}

/// The reporter sees their own title, never the description.
pub async fn by_ref(c: &Connection, ref_id: i64) -> anyhow::Result<Vec<MyTicket>> {
    crate::platform::db::query_all(c, "SELECT number, title, stage, close_reason, created_at, updated_at FROM tickets WHERE ref_id = ?1 ORDER BY created_at DESC LIMIT 100", [ref_id], |r| Ok(MyTicket {
            number: r.get(0)?,
            title: r.get(1)?,
            stage: r.get(2)?,
            close_reason: r.get(3)?,
            created_at: r.get(4)?,
            updated_at: r.get(5)?,
        })).await
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

/// The first `caps::DIGEST_LINES` triage tickets after `after_id`, and how many there are in all.
pub async fn new_in_triage_after(
    c: &Connection,
    project_id: i64,
    after_id: i64,
) -> anyhow::Result<(Vec<(i64, String)>, i64)> {
    let rows = crate::platform::db::query_all(c, "SELECT number, title FROM tickets WHERE project_id = ?1 AND stage = 'triage' AND id > ?2 ORDER BY number LIMIT ?3", (project_id, after_id, caps::DIGEST_LINES), |r| Ok((r.get(0)?, r.get(1)?))).await?;
    let total = crate::platform::db::query_i64(
        c,
        "SELECT count(*) FROM tickets WHERE project_id = ?1 AND stage = 'triage' AND id > ?2",
        (project_id, after_id),
    )
    .await?
    .unwrap_or(0);
    Ok((rows, total))
}

pub async fn max_id(c: &Connection) -> anyhow::Result<i64> {
    Ok(
        crate::platform::db::query_i64(c, "SELECT coalesce(max(id), 0) FROM tickets", ())
            .await?
            .unwrap_or(0),
    )
}

/// Tickets of a project not yet done, for its tab count.
pub async fn open_count(c: &Connection, project_id: i64) -> anyhow::Result<i64> {
    Ok(crate::platform::db::query_i64(
        c,
        "SELECT count(*) FROM tickets WHERE project_id = ?1 AND stage != 'done'",
        [project_id],
    )
    .await?
    .unwrap_or(0))
}
