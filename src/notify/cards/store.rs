//! SQL for alert_cards.

use turso::Connection;

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
    pub enabled: bool,
    /// An `email` card's addresses.
    pub emails: Vec<String>,
    pub url: Option<String>,
    /// Telegram's chat ID or PagerDuty's routing key.
    pub key: Option<String>,
    /// The owner's two cards: only their switch changes.
    pub fixed: bool,
}

/// A card's settings as they are stored.
pub struct Saved {
    pub kind: String,
    pub name: String,
    pub critical: bool,
    pub warning: bool,
    pub emails: Vec<String>,
    pub url: Option<String>,
    pub key: Option<String>,
}

const COLS: &str = "id, kind, name, critical, warning, enabled, emails, url, key";

/// Addresses kept one a line; a list from before cards may use commas.
pub fn split_emails(raw: &str) -> Vec<String> {
    raw.split(['\n', ','])
        .map(str::trim)
        .filter(|e| !e.is_empty())
        .map(str::to_owned)
        .collect()
}

fn row(r: &turso::Row) -> turso::Result<AlertCard> {
    let kind: String = r.get(1)?;
    Ok(AlertCard {
        id: r.get(0)?,
        fixed: kind == "owner_email",
        kind,
        name: r.get(2)?,
        critical: r.get::<i64>(3)? == 1,
        warning: r.get::<i64>(4)? == 1,
        enabled: r.get::<i64>(5)? == 1,
        emails: split_emails(&r.get::<String>(6)?),
        url: r.get(7)?,
        key: r.get(8)?,
    })
}

/// The most rows a project holds: its own cards and the owner's two.
const ALL: i64 = caps::ALERT_CARDS + 2;

/// The owner's cards first, critical before warning, then the project's own as they were added.
pub async fn list(c: &Connection, project_id: i64) -> anyhow::Result<Vec<AlertCard>> {
    query_all(
        c,
        &format!(
            "SELECT {COLS} FROM alert_cards WHERE project_id = ?1
             ORDER BY kind <> 'owner_email', warning, id LIMIT ?2"
        ),
        (project_id, ALL),
        row,
    )
    .await
}

/// The cards that are on and listen to `severity`.
pub async fn listening(
    c: &Connection,
    project_id: i64,
    severity: &str,
) -> anyhow::Result<Vec<AlertCard>> {
    query_all(
        c,
        &format!(
            "SELECT {COLS} FROM alert_cards WHERE project_id = ?1 AND enabled = 1
             AND (CASE ?2 WHEN 'critical' THEN critical ELSE warning END) = 1
             ORDER BY id LIMIT ?3"
        ),
        (project_id, severity, ALL),
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

/// The project's own cards, which the cap counts.
pub async fn own_count(c: &Connection, project_id: i64) -> anyhow::Result<i64> {
    Ok(crate::platform::db::query_i64(
        c,
        "SELECT count(*) FROM alert_cards WHERE project_id = ?1 AND kind <> 'owner_email'",
        [project_id],
    )
    .await?
    .unwrap_or(0))
}

pub async fn insert(c: &Connection, project_id: i64, s: &Saved) -> anyhow::Result<i64> {
    c.execute(
        "INSERT INTO alert_cards (project_id, kind, name, critical, warning, emails, url, key)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        (
            project_id,
            s.kind.as_str(),
            s.name.as_str(),
            s.critical,
            s.warning,
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
            "UPDATE alert_cards SET kind = ?3, name = ?4, critical = ?5, warning = ?6, emails = ?7,
               url = ?8, key = ?9
             WHERE project_id = ?1 AND id = ?2 AND kind <> 'owner_email'",
            (
                project_id,
                id,
                s.kind.as_str(),
                s.name.as_str(),
                s.critical,
                s.warning,
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

/// A new project's two cards: the owner's email for critical and for warning, both on.
pub async fn create_owner_cards(c: &Connection, project_id: i64) -> anyhow::Result<()> {
    c.execute(
        "INSERT INTO alert_cards (project_id, kind, name, critical, warning) VALUES
           (?1, 'owner_email', 'Owner email', 1, 0), (?1, 'owner_email', 'Owner email', 0, 1)",
        [project_id],
    )
    .await?;
    Ok(())
}
