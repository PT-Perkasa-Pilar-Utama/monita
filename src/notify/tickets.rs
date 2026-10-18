//! Ticket alerts, apart from incident severities: the owner's daily digest and a webhook per
//! new ticket, each switched on its own.

use turso::Connection;

use super::{RouteError, store, store::TicketAlerts, ticket_json};
use crate::platform::{db::Db, now};

pub async fn ticket_alerts(db: &Db, project_id: i64) -> anyhow::Result<TicketAlerts> {
    store::ticket_alerts(&db.read().await?, project_id).await
}

/// A webhook switched on needs its URL; a URL kept while off waits for the next time.
pub async fn save_ticket_alerts(
    db: &Db,
    project_id: i64,
    digest: bool,
    webhook_raw: &str,
    use_webhook: bool,
) -> Result<(), RouteError> {
    let webhook = webhook_raw.trim();
    if (use_webhook || !webhook.is_empty()) && !crate::platform::valid_http_url(webhook) {
        return Err(RouteError::BadWebhook);
    }
    let a = TicketAlerts {
        enabled: true,
        digest,
        webhook_url: (!webhook.is_empty()).then(|| webhook.to_string()),
        use_webhook,
    };
    db.write(async |c| store::save_ticket_alerts(c, project_id, &a).await)
        .await?;
    Ok(())
}

/// AC-30.03: one webhook call per new ticket from a reporter, when the ticket webhook is on.
pub async fn send_ticket(
    c: &Connection,
    project_id: i64,
    number: i64,
    title: &str,
    base_url: &str,
) -> anyhow::Result<()> {
    let a = store::ticket_alerts(c, project_id).await?;
    let (true, Some(url)) = (a.enabled && a.use_webhook, a.webhook_url.as_deref()) else {
        return Ok(());
    };
    let Some((name, slug)) = crate::projects::name_slug(c, project_id).await? else {
        return Ok(());
    };
    let body = ticket_json(&name, &slug, number, title, base_url, now());
    store::queue(c, "webhook", url, None, &body.to_string(), now()).await
}

/// The test button of Ticket alerts: one `test` event to the ticket webhook. Returns 0 when it
/// is off. A test never looks like a ticket, so a tool that files tickets from the webhook skips it.
pub async fn send_ticket_test(db: &Db, project_id: i64, base_url: &str) -> anyhow::Result<usize> {
    db.write(async |c| {
        let a = store::ticket_alerts(c, project_id).await?;
        let (true, Some(url)) = (a.enabled && a.use_webhook, a.webhook_url.as_deref()) else {
            return Ok(0);
        };
        let Some((name, slug)) = crate::projects::name_slug(c, project_id).await? else {
            return Ok(0);
        };
        let at = now();
        let text = format!("[TEST] {name}: new-ticket alerts reach this channel.");
        let json = serde_json::json!({ "text": text, "event": "test", "project": slug, "at": at, "link": format!("{base_url}/projects/{slug}/settings/alerts") });
        store::queue(c, "webhook", url, None, &json.to_string(), at).await?;
        Ok(1)
    })
    .await
}
