//! Ticket alerts, apart from alert cards: the owner's daily digest, a webhook per new ticket, and
//! the same webhook per column move, each switched on its own. The webhook speaks a format as a
//! card does (v1.1 Q36), PagerDuty aside: a ticket is no incident, so a trigger would never
//! resolve.

use turso::Connection;

use super::{
    AlertError,
    card::{Card, Tone},
    compose::Composed,
    format, store,
    store::TicketAlerts,
    telegram, ticket_json,
};
use crate::platform::{db::Db, now};

/// The formats the ticket webhook takes.
const TICKET_FORMATS: [&str; 5] = ["json", "discord", "telegram", "telegram_bot", "ntfy"];

/// What the Ticket alerts form sends.
pub struct TicketForm<'a> {
    pub digest: bool,
    pub webhook_url: &'a str,
    pub use_webhook: bool,
    pub webhook_moves: bool,
    /// One of `TICKET_FORMATS`; empty reads as `json`.
    pub webhook_format: &'a str,
    /// A Telegram chat ID.
    pub webhook_key: &'a str,
}

/// A ticket event laid out for every format: the JSON body as before, and a card for Discord and
/// Telegram.
fn ticket_alert(
    name: &str,
    json: serde_json::Value,
    label: &'static str,
    title: String,
    facts: Vec<(&'static str, String)>,
    link_label: &'static str,
) -> Composed {
    let text = json["text"].as_str().unwrap_or_default().to_string();
    let link = json["ticket_url"]
        .as_str()
        .or(json["link"].as_str())
        .unwrap_or_default()
        .to_string();
    Composed {
        card: Card {
            tone: Tone::Info,
            label,
            title,
            project: name.to_string(),
            facts,
            link_label,
        },
        subject: text.clone(),
        text,
        json,
        link,
        dedup: "ticket".into(),
        resolves: false,
    }
}

/// The webhook call, when there is a URL. Runs inside the caller's write.
async fn queue(
    c: &Connection,
    a: &TicketAlerts,
    slug: &str,
    msg: &Composed,
    at: i64,
) -> anyhow::Result<bool> {
    let Some(url) = a.webhook_url.as_deref() else {
        return Ok(false);
    };
    super::queue_webhook(
        c,
        &a.webhook_format,
        url,
        a.webhook_key.as_deref(),
        slug,
        msg,
        at,
    )
    .await?;
    Ok(true)
}

pub async fn ticket_alerts(db: &Db, project_id: i64) -> anyhow::Result<TicketAlerts> {
    store::ticket_alerts(&db.read().await?, project_id).await
}

/// A webhook switched on, for new tickets or for moves, needs its URL. A URL kept while both are
/// off waits for the next time.
pub async fn save_ticket_alerts(
    db: &Db,
    project_id: i64,
    f: &TicketForm<'_>,
) -> Result<(), AlertError> {
    let fmt = Some(f.webhook_format.trim())
        .filter(|v| !v.is_empty())
        .unwrap_or("json");
    if !TICKET_FORMATS.contains(&fmt) {
        return Err(AlertError::BadFormat);
    }
    let needs = f.use_webhook || f.webhook_moves;
    let (url, key) = if needs || !f.webhook_url.trim().is_empty() || fmt == "telegram_bot" {
        format::check(fmt, f.webhook_url, f.webhook_key)?
    } else {
        (None, None)
    };
    if needs && url.is_none() {
        return Err(AlertError::BadWebhook);
    }
    if needs && fmt == "telegram_bot" && telegram::bot_name(db).await?.is_none() {
        return Err(AlertError::NoBot);
    }
    let a = TicketAlerts {
        // Not written: saving keeps the on and off switch as it is (`set_enabled` owns it).
        enabled: true,
        digest: f.digest,
        webhook_url: url,
        use_webhook: f.use_webhook,
        webhook_moves: f.webhook_moves,
        webhook_format: fmt.to_string(),
        webhook_key: key,
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
    if !(a.enabled && a.use_webhook) {
        return Ok(());
    }
    let Some((name, slug)) = crate::projects::name_slug(c, project_id).await? else {
        return Ok(());
    };
    let at = now();
    let json = ticket_json(&name, &slug, number, title, base_url, at);
    let msg = ticket_alert(
        &name,
        json,
        "New ticket",
        format!("#{number} {title}"),
        Vec::new(),
        "Open the ticket",
    );
    queue(c, &a, &slug, &msg, at).await?;
    Ok(())
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
    if !(a.enabled && a.webhook_moves) {
        return Ok(());
    }
    let Some((name, slug)) = crate::projects::name_slug(c, project_id).await? else {
        return Ok(());
    };
    let at = now();
    let json = ticket_moved_json(&name, &slug, m, base_url, at);
    let mut to = label(m.to).to_string();
    if let Some(r) = m.close_reason {
        to = format!("{to} ({})", label(r));
    }
    let facts = vec![("From", label(m.from).to_string()), ("To", to)];
    let title = format!("#{} {}", m.number, m.title);
    let msg = ticket_alert(&name, json, "Ticket moved", title, facts, "Open the ticket");
    queue(c, &a, &slug, &msg, at).await?;
    Ok(())
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
        if !(a.enabled && (a.use_webhook || a.webhook_moves)) {
            return Ok(0);
        }
        let Some((name, slug)) = crate::projects::name_slug(c, project_id).await? else {
            return Ok(0);
        };
        let at = now();
        let text = format!("[TEST] {name}: ticket alerts reach this channel.");
        let json = serde_json::json!({ "text": text, "event": "test", "project": slug, "at": at, "link": format!("{base_url}/projects/{slug}/settings/alerts") });
        let title = "Ticket alerts reach this chat".to_string();
        let msg = ticket_alert(&name, json, "Test", title, Vec::new(), "Open the alert settings");
        Ok(usize::from(queue(c, &a, &slug, &msg, at).await?))
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
