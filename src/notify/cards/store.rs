//! SQL for alert_cards.

use crate::platform::sql::Connection;

use crate::platform::{caps, db::query_all};

/// One alert card as the page shows it. A supervisor gets `(hidden)` for the URL and the key.
#[derive(serde::Serialize, Clone, Debug, PartialEq, Eq)]
pub struct AlertCard {
    pub id: i64,
    /// `owner_email`, `email`, or a webhook format (`format::FORMATS`).
    pub kind: String,
    pub name: String,
    pub critical: bool,
    pub warning: bool,
    pub new_tickets: bool,
    pub ticket_moves: bool,
    /// The owner's daily digest of new tickets; only an owner card has it.
    pub digest: bool,
    /// `incidents` or `tickets`, from what the card listens to. A card never hears both.
    pub section: &'static str,
    pub enabled: bool,
    /// An `email` card's addresses.
    pub emails: Vec<String>,
    pub url: Option<String>,
    /// Telegram's chat ID or PagerDuty's routing key.
    pub key: Option<String>,
    /// The owner's cards: only their switch changes.
    pub fixed: bool,
}

/// A card's settings as they are stored.
pub struct Saved {
    pub kind: String,
    pub name: String,
    pub critical: bool,
    pub warning: bool,
    pub new_tickets: bool,
    pub ticket_moves: bool,
    pub emails: Vec<String>,
    pub url: Option<String>,
    pub key: Option<String>,
}

impl Saved {
    pub fn incidents(&self) -> bool {
        self.critical || self.warning
    }
}

const COLS: &str = "id, kind, name, critical, warning, new_tickets, ticket_moves, digest, enabled, emails, url, key";

/// What a card listens to. Each names one column, so no caller text reaches the SQL.
#[derive(Clone, Copy)]
pub enum Listen {
    Critical,
    Warning,
    NewTickets,
    TicketMoves,
    Digest,
}

impl Listen {
    fn column(self) -> &'static str {
        match self {
            Listen::Critical => "critical",
            Listen::Warning => "warning",
            Listen::NewTickets => "new_tickets",
            Listen::TicketMoves => "ticket_moves",
            Listen::Digest => "digest",
        }
    }
}

/// Addresses kept one a line; a list from before cards may use commas.
pub fn split_emails(raw: &str) -> Vec<String> {
    raw.split(['\n', ','])
        .map(str::trim)
        .filter(|e| !e.is_empty())
        .map(str::to_owned)
        .collect()
}

fn row(r: &crate::platform::sql::Row) -> crate::platform::sql::Result<AlertCard> {
    let kind: String = r.get(1)?;
    let flag = |i: usize| r.get::<i64>(i).map(|v| v == 1);
    let (critical, warning) = (flag(3)?, flag(4)?);
    Ok(AlertCard {
        id: r.get(0)?,
        fixed: kind == "owner_email",
        kind,
        name: r.get(2)?,
        critical,
        warning,
        new_tickets: flag(5)?,
        ticket_moves: flag(6)?,
        digest: flag(7)?,
        section: if critical || warning {
            "incidents"
        } else {
            "tickets"
        },
        enabled: flag(8)?,
        emails: split_emails(&r.get::<String>(9)?),
        url: r.get(10)?,
        key: r.get(11)?,
    })
}

/// The SQL that picks a section's cards.
const INCIDENTS: &str = "(critical = 1 OR warning = 1)";

/// The most rows a project holds: its own cards in each section and the owner's five.
const ALL: i64 = 2 * caps::ALERT_CARDS + 5;

/// Incident cards, then ticket cards. In each, the owner's cards come first in a fixed order,
/// then the project's own as they were added.
pub async fn list(c: &Connection, project_id: i64) -> anyhow::Result<Vec<AlertCard>> {
    query_all(
        c,
        &format!(
            "SELECT {COLS} FROM alert_cards WHERE project_id = ?1
             ORDER BY NOT {INCIDENTS}, kind <> 'owner_email',
               CASE kind WHEN 'owner_email' THEN warning + new_tickets + 2 * ticket_moves END, id
             LIMIT ?2"
        ),
        (project_id, ALL),
        row,
    )
    .await
}

/// The cards that are on and listen to `what`.
pub async fn listening(
    c: &Connection,
    project_id: i64,
    what: Listen,
) -> anyhow::Result<Vec<AlertCard>> {
    query_all(
        c,
        &format!(
            "SELECT {COLS} FROM alert_cards WHERE project_id = ?1 AND enabled = 1 AND {} = 1
             ORDER BY id LIMIT ?2",
            what.column()
        ),
        (project_id, ALL),
        row,
    )
    .await
}

pub async fn one(c: &Connection, project_id: i64, id: i64) -> anyhow::Result<Option<AlertCard>> {
    Ok(query_all(
        c,
        &format!("SELECT {COLS} FROM alert_cards WHERE project_id = ?1 AND id = ?2"),
        (project_id, id),
        row,
    )
    .await?
    .pop())
}

/// The project's own cards in one section, which the cap counts.
pub async fn own_count(c: &Connection, project_id: i64, incidents: bool) -> anyhow::Result<i64> {
    Ok(crate::platform::db::query_i64(
        c,
        &format!(
            "SELECT count(*) FROM alert_cards WHERE project_id = ?1 AND kind <> 'owner_email'
             AND {INCIDENTS} = ?2"
        ),
        (project_id, incidents),
    )
    .await?
    .unwrap_or(0))
}

pub async fn insert(c: &Connection, project_id: i64, s: &Saved) -> anyhow::Result<i64> {
    c.execute(
        "INSERT INTO alert_cards
           (project_id, kind, name, critical, warning, new_tickets, ticket_moves, emails, url, key)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        (
            project_id,
            s.kind.as_str(),
            s.name.as_str(),
            s.critical,
            s.warning,
            s.new_tickets,
            s.ticket_moves,
            s.emails.join("\n"),
            s.url.as_deref(),
            s.key.as_deref(),
        ),
    )
    .await?;
    Ok(c.last_insert_rowid())
}

/// Changes one of the project's own cards; the owner's cards never change here.
pub async fn update(c: &Connection, project_id: i64, id: i64, s: &Saved) -> anyhow::Result<bool> {
    let n = c
        .execute(
            "UPDATE alert_cards SET kind = ?3, name = ?4, critical = ?5, warning = ?6,
               new_tickets = ?7, ticket_moves = ?8, emails = ?9, url = ?10, key = ?11
             WHERE project_id = ?1 AND id = ?2 AND kind <> 'owner_email'",
            (
                project_id,
                id,
                s.kind.as_str(),
                s.name.as_str(),
                s.critical,
                s.warning,
                s.new_tickets,
                s.ticket_moves,
                s.emails.join("\n"),
                s.url.as_deref(),
                s.key.as_deref(),
            ),
        )
        .await?;
    Ok(n > 0)
}

pub async fn delete(c: &Connection, project_id: i64, id: i64) -> anyhow::Result<bool> {
    let n = c
        .execute(
            "DELETE FROM alert_cards WHERE project_id = ?1 AND id = ?2 AND kind <> 'owner_email'",
            (project_id, id),
        )
        .await?;
    Ok(n > 0)
}

pub async fn set_enabled(
    c: &Connection,
    project_id: i64,
    id: i64,
    on: bool,
) -> anyhow::Result<bool> {
    let n = c
        .execute(
            "UPDATE alert_cards SET enabled = ?3 WHERE project_id = ?1 AND id = ?2",
            (project_id, id, on),
        )
        .await?;
    Ok(n > 0)
}

/// A new project's owner cards: email for critical and for warning, both on (v1.1 Q34), and for
/// the daily digest, each new ticket and each move, all off (v1.1 Q37).
pub async fn create_owner_cards(c: &Connection, project_id: i64) -> anyhow::Result<()> {
    c.execute(
        "INSERT INTO alert_cards
           (project_id, kind, name, critical, warning, digest, new_tickets, ticket_moves, enabled)
         VALUES (?1, 'owner_email', 'Owner email', 1, 0, 0, 0, 0, 1),
                (?1, 'owner_email', 'Owner email', 0, 1, 0, 0, 0, 1),
                (?1, 'owner_email', 'Owner email', 0, 0, 1, 0, 0, 0),
                (?1, 'owner_email', 'Owner email', 0, 0, 0, 1, 0, 0),
                (?1, 'owner_email', 'Owner email', 0, 0, 0, 0, 1, 0)",
        [project_id],
    )
    .await?;
    Ok(())
}
