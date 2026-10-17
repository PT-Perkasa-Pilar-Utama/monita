//! SQL for alert_routes and outbox.

use turso::Connection;

pub struct Route {
    pub severity: String,
    pub emails: Vec<String>,
    pub webhook_url: Option<String>,
    pub use_webhook: bool,
}

pub struct OutboxRow {
    pub id: i64,
    pub channel: String,
    pub recipient: String,
    pub subject: Option<String>,
    pub body: String,
    pub attempts: i64,
}

pub enum Outcome {
    Sent,
    Retry(i64, String),
    Failed(String),
}

fn route_row(r: &turso::Row) -> turso::Result<Route> {
    let emails: String = r.get(1)?;
    Ok(Route {
        severity: r.get(0)?,
        emails: emails
            .lines()
            .filter(|l| !l.is_empty())
            .map(str::to_owned)
            .collect(),
        webhook_url: r.get(2)?,
        use_webhook: r.get::<i64>(3)? == 1,
    })
}

pub async fn routes(c: &Connection, project_id: i64) -> anyhow::Result<Vec<Route>> {
    crate::platform::db::query_all(c, "SELECT severity, emails, webhook_url, use_webhook FROM alert_routes WHERE project_id = ?1 ORDER BY severity LIMIT 2", [project_id], route_row).await
}

pub async fn route(
    c: &Connection,
    project_id: i64,
    severity: &str,
) -> anyhow::Result<Option<Route>> {
    let mut rows = c
        .query(
            "SELECT severity, emails, webhook_url, use_webhook FROM alert_routes WHERE project_id = ?1 AND severity = ?2",
            (project_id, severity),
        )
        .await?;
    Ok(match rows.next().await? {
        Some(r) => Some(route_row(&r)?),
        None => None,
    })
}

pub async fn save_route(
    c: &Connection,
    project_id: i64,
    severity: &str,
    emails: &str,
    webhook: Option<&str>,
    use_webhook: bool,
) -> anyhow::Result<()> {
    c.execute(
        "INSERT INTO alert_routes (project_id, severity, emails, webhook_url, use_webhook) VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT (project_id, severity) DO UPDATE SET emails = excluded.emails, webhook_url = excluded.webhook_url, use_webhook = excluded.use_webhook",
        (project_id, severity, emails, webhook, use_webhook),
    )
    .await?;
    Ok(())
}

pub async fn queue(
    c: &Connection,
    channel: &str,
    recipient: &str,
    subject: Option<&str>,
    body: &str,
    at: i64,
) -> anyhow::Result<()> {
    c.execute(
        "INSERT INTO outbox (channel, recipient, subject, body, next_attempt_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        (channel, recipient, subject, body, at),
    )
    .await?;
    Ok(())
}

pub async fn pending_count(c: &Connection) -> anyhow::Result<i64> {
    Ok(crate::platform::db::query_i64(
        c,
        "SELECT count(*) FROM outbox WHERE sent_at IS NULL AND failed_at IS NULL",
        (),
    )
    .await?
    .unwrap_or(0))
}

pub async fn any_due(c: &Connection, at: i64) -> anyhow::Result<bool> {
    let n = crate::platform::db::query_i64(
        c,
        "SELECT EXISTS (SELECT 1 FROM outbox WHERE sent_at IS NULL AND failed_at IS NULL AND next_attempt_at <= ?1)",
        [at],
    )
    .await?;
    Ok(n == Some(1))
}

/// Claims due rows by pushing `next_attempt_at` to `lease_until`, so two delivery passes never
/// send one row twice. A crash mid-send leaves the lease to expire and the row is retried.
pub async fn claim_due(
    c: &Connection,
    at: i64,
    limit: i64,
    lease_until: i64,
) -> anyhow::Result<Vec<OutboxRow>> {
    let mut out = Vec::new();
    {
        let mut rows = c
            .query(
                "SELECT id, channel, recipient, subject, body, attempts FROM outbox
                 WHERE sent_at IS NULL AND failed_at IS NULL AND next_attempt_at <= ?1 ORDER BY next_attempt_at, id LIMIT ?2",
                (at, limit),
            )
            .await?;
        while let Some(r) = rows.next().await? {
            out.push(OutboxRow {
                id: r.get(0)?,
                channel: r.get(1)?,
                recipient: r.get(2)?,
                subject: r.get(3)?,
                body: r.get(4)?,
                attempts: r.get(5)?,
            });
        }
    }
    for row in &out {
        c.execute(
            "UPDATE outbox SET next_attempt_at = ?1 WHERE id = ?2",
            (lease_until, row.id),
        )
        .await?;
    }
    Ok(out)
}

pub async fn create_routes(c: &Connection, project_id: i64) -> anyhow::Result<()> {
    c.execute(
        "INSERT INTO alert_routes (project_id, severity, use_webhook) VALUES (?1, 'critical', 1), (?1, 'warning', 0)",
        [project_id],
    )
    .await?;
    Ok(())
}

/// Deletes up to `limit` finished rows older than `cutoff`; returns how many it deleted.
pub async fn purge_delivered(c: &Connection, cutoff: i64, limit: i64) -> anyhow::Result<u64> {
    Ok(c.execute(
        "DELETE FROM outbox WHERE rowid IN (SELECT rowid FROM outbox WHERE created_at < ?1 AND (sent_at IS NOT NULL OR failed_at IS NOT NULL) LIMIT ?2)",
        (cutoff, limit),
    )
    .await?)
}

/// A finished row loses its body: an invite email carries a live sign-up link (SEC-20).
pub async fn mark(c: &Connection, id: i64, outcome: &Outcome, at: i64) -> anyhow::Result<()> {
    match outcome {
        Outcome::Sent => c.execute("UPDATE outbox SET sent_at = ?1, attempts = attempts + 1, body = '' WHERE id = ?2", (at, id)).await?,
        Outcome::Retry(next, err) => {
            c.execute("UPDATE outbox SET attempts = attempts + 1, next_attempt_at = ?1, last_error = ?2 WHERE id = ?3", (*next, err.as_str(), id))
                .await?
        }
        Outcome::Failed(err) => c.execute("UPDATE outbox SET attempts = attempts + 1, failed_at = ?1, last_error = ?2, body = '' WHERE id = ?3", (at, err.as_str(), id)).await?,
    };
    Ok(())
}

/// Emails created since `since`: (sent, failed for good, still waiting), one aggregate row.
pub async fn email_counts(c: &Connection, since: i64) -> anyhow::Result<(i64, i64, i64)> {
    let rows = crate::platform::db::query_all(
        c,
        "SELECT coalesce(sum(sent_at IS NOT NULL), 0), coalesce(sum(failed_at IS NOT NULL), 0),
                coalesce(sum(sent_at IS NULL AND failed_at IS NULL), 0)
         FROM outbox WHERE channel = 'email' AND created_at >= ?1",
        [since],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )
    .await?;
    Ok(rows.into_iter().next().unwrap_or((0, 0, 0)))
}

/// The newest email delivery: when it was sent, or the error of the newest attempt that failed.
pub async fn email_latest(c: &Connection) -> anyhow::Result<(Option<i64>, Option<(String, i64)>)> {
    let sent = crate::platform::db::query_i64(
        c,
        "SELECT max(sent_at) FROM outbox WHERE channel = 'email'",
        (),
    )
    .await?;
    let error = crate::platform::db::query_all(
        c,
        "SELECT last_error, coalesce(failed_at, created_at) FROM outbox
         WHERE channel = 'email' AND last_error IS NOT NULL ORDER BY id DESC LIMIT 1",
        (),
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .await?;
    Ok((sent, error.into_iter().next()))
}
