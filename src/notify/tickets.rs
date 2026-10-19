//! Ticket alerts, apart from incident severities: the owner's daily digest, a webhook per new
//! ticket, and the same webhook per column move, each switched on its own.

use turso::Connection;

use super::{RouteError, store, store::TicketAlerts, ticket_json};
use crate::platform::{db::Db, now};

pub async fn ticket_alerts(db: &Db, project_id: i64) -> anyhow::Result<TicketAlerts> {
    store::ticket_alerts(&db.read().await?, project_id).await
}

/// A webhook switched on, for new tickets or for moves, needs its URL; a URL kept while both are
/// off waits for the next time.
pub async fn save_ticket_alerts(
    db: &Db,
    project_id: i64,
    digest: bool,
    webhook_raw: &str,
    use_webhook: bool,
    webhook_moves: bool,
) -> Result<(), RouteError> {
    let webhook = webhook_raw.trim();
    let needs_url = use_webhook || webhook_moves;
    if (needs_url || !webhook.is_empty()) && !crate::platform::valid_http_url(webhook) {
        return Err(RouteError::BadWebhook);
    }
    let a = TicketAlerts {
        // Not written: saving keeps the on and off switch as it is (`set_enabled` owns it).
        enabled: true,
        digest,
        webhook_url: (!webhook.is_empty()).then(|| webhook.to_string()),
        use_webhook,
        webhook_moves,
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

/// A ticket that changed column, for the move webhook.
pub struct Moved<'a> {
    pub number: i64,
    pub title: &'a str,
    pub from: &'a str,
    pub to: &'a str,
    pub close_reason: Option<&'a str>,
}

/// One webhook call per column move, when the move webhook is on. A move to the same column
/// sends nothing.
pub async fn send_ticket_moved(
    c: &Connection,
    project_id: i64,
    m: &Moved<'_>,
    base_url: &str,
) -> anyhow::Result<()> {
    if m.from == m.to {
        return Ok(());
    }
    let a = store::ticket_alerts(c, project_id).await?;
    let (true, Some(url)) = (a.enabled && a.webhook_moves, a.webhook_url.as_deref()) else {
        return Ok(());
    };
    let Some((name, slug)) = crate::projects::name_slug(c, project_id).await? else {
        return Ok(());
    };
    let at = now();
    let body = ticket_moved_json(&name, &slug, m, base_url, at);
    store::queue(c, "webhook", url, None, &body.to_string(), at).await
}

/// The webhook body for a ticket move.
pub(super) fn ticket_moved_json(
    name: &str,
    slug: &str,
    m: &Moved<'_>,
    base_url: &str,
    at: i64,
) -> serde_json::Value {
    let mut to = label(m.to).to_string();
    if let Some(r) = m.close_reason {
        to = format!("{to} ({})", label(r));
    }
    let number = m.number;
    serde_json::json!({
        "text": format!("[TICKET] {name} #{number} moved from {} to {to}: {}", label(m.from), super::chat_safe(m.title)),
        "event": "ticket.moved",
        "project": slug,
        "ticket_number": number,
        "title": m.title,
        "from": m.from,
        "to": m.to,
        "close_reason": m.close_reason,
        "ticket_url": format!("{base_url}/projects/{slug}/tickets/{number}"),
        "at": at,
    })
}

/// The board's words for a stage or a close reason.
fn label(value: &str) -> &str {
    match value {
        "triage" => "Triage",
        "backlog" => "Backlog",
        "in_progress" => "In progress",
        "done" => "Done",
        "fixed" => "Fixed",
        "wont_fix" => "Won't fix",
        "duplicate" => "Duplicate",
        other => other,
    }
}

/// The test button of Ticket alerts: one `test` event to the ticket webhook. Returns 0 when it
/// is off. A test never looks like a ticket, so a tool that files tickets from the webhook skips it.
pub async fn send_ticket_test(db: &Db, project_id: i64, base_url: &str) -> anyhow::Result<usize> {
    db.write(async |c| {
        let a = store::ticket_alerts(c, project_id).await?;
        let on = a.use_webhook || a.webhook_moves;
        let (true, Some(url)) = (a.enabled && on, a.webhook_url.as_deref()) else {
            return Ok(0);
        };
        let Some((name, slug)) = crate::projects::name_slug(c, project_id).await? else {
            return Ok(0);
        };
        let at = now();
        let text = format!("[TEST] {name}: ticket alerts reach this channel.");
        let json = serde_json::json!({ "text": text, "event": "test", "project": slug, "at": at, "link": format!("{base_url}/projects/{slug}/settings/alerts") });
        store::queue(c, "webhook", url, None, &json.to_string(), at).await?;
        Ok(1)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reporters_title_cannot_add_a_link_or_mention_to_the_chat_text() {
        let title = "<!channel> pay at <https://evil.example|bank> & more";
        let created = ticket_json("Payments", "pay", 2, title, "https://m.co", 0);
        assert_eq!(
            created["text"],
            "[TICKET] Payments #2: &lt;!channel&gt; pay at &lt;https://evil.example|bank&gt; &amp; more"
        );
        assert_eq!(
            created["title"], title,
            "code that reads the JSON gets the words"
        );
        let m = Moved {
            number: 2,
            title,
            from: "triage",
            to: "backlog",
            close_reason: None,
        };
        let moved = ticket_moved_json("Payments", "pay", &m, "https://m.co", 0);
        assert!(
            moved["text"]
                .as_str()
                .unwrap()
                .ends_with("&lt;!channel&gt; pay at &lt;https://evil.example|bank&gt; &amp; more")
        );
    }
}
