//! Ticket alerts go to the ticket cards (v1.1 Q37): one per new ticket from a reporter and one
//! per column move, each to the cards that listen to it. The daily digest is `digest`.

use crate::platform::sql::Connection;

use super::{
    card::{Card, Tone},
    cards::{self, Listen},
    compose::Composed,
    ticket_json,
};
use crate::platform::now;

/// A ticket event laid out for every kind of card: an email, the JSON body as before, and a card
/// for Discord and Telegram. The email is plain text, so it gets the title as typed.
fn ticket_alert(
    name: &str,
    json: serde_json::Value,
    label: &'static str,
    title: String,
    facts: Vec<(&'static str, String)>,
    subject: String,
) -> Composed {
    let link = json["ticket_url"].as_str().unwrap_or_default().to_string();
    let lines: String = facts.iter().map(|(k, v)| format!("{k}: {v}\n")).collect();
    Composed {
        text: format!("{title}\n{lines}\nOpen the ticket: {link}\n"),
        card: Card {
            tone: Tone::Info,
            label,
            title,
            project: name.to_string(),
            facts,
            link_label: "Open the ticket",
        },
        subject,
        json,
        link,
        dedup: "ticket".into(),
        resolves: false,
    }
}

/// AC-30.03: a new ticket from a reporter, to every ticket card that listens to new tickets.
pub async fn send_ticket(
    c: &Connection,
    project_id: i64,
    number: i64,
    title: &str,
    base_url: &str,
) -> anyhow::Result<()> {
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
        format!("[TICKET] {name} / #{number} {title}"),
    );
    cards::queue_alert(c, project_id, &slug, Listen::NewTickets, &msg).await?;
    Ok(())
}

/// A ticket that changed column.
pub struct Moved<'a> {
    pub number: i64,
    pub title: &'a str,
    pub from: &'a str,
    pub to: &'a str,
    pub close_reason: Option<&'a str>,
}

/// A column move, to every ticket card that listens to moves. A move to the same column sends
/// nothing.
pub async fn send_ticket_moved(
    c: &Connection,
    project_id: i64,
    m: &Moved<'_>,
    base_url: &str,
) -> anyhow::Result<()> {
    if m.from == m.to {
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
    let subject = format!("[TICKET] {name} / #{} moved to {to}", m.number);
    let facts = vec![("From", label(m.from).to_string()), ("To", to)];
    let title = format!("#{} {}", m.number, m.title);
    let msg = ticket_alert(&name, json, "Ticket moved", title, facts, subject);
    cards::queue_alert(c, project_id, &slug, Listen::TicketMoves, &msg).await?;
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
