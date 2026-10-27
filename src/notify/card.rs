//! An alert laid out for a chat app that can style it: Discord as an embed, Telegram as HTML.
//! Every alert builds one `Card` beside its plain text, which email, JSON, ntfy and PagerDuty use.

use serde_json::{Value, json};

/// What the alert means, which sets its colour and its mark.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Tone {
    Critical,
    Warning,
    Good,
    Info,
}

impl Tone {
    /// Critical or warning, from a severity.
    pub(super) fn of(severity: &str) -> Self {
        if severity == "critical" {
            Tone::Critical
        } else {
            Tone::Warning
        }
    }

    fn mark(self) -> &'static str {
        match self {
            Tone::Critical => "🔴",
            Tone::Warning => "🟠",
            Tone::Good => "🟢",
            Tone::Info => "🔵",
        }
    }

    /// Discord's embed colour, as an RGB integer.
    fn colour(self) -> u32 {
        match self {
            Tone::Critical => 0xE5484D,
            Tone::Warning => 0xF76B15,
            Tone::Good => 0x30A46C,
            Tone::Info => 0x0090FF,
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct Card {
    pub(super) tone: Tone,
    /// One word: Critical, Warning, Resolved, Down, Up, Certificate or Test.
    pub(super) label: &'static str,
    /// What happened, without the project: "API health is down".
    pub(super) title: String,
    /// The project's name.
    pub(super) project: String,
    /// Named lines under the title, such as the cause or how long it was down.
    pub(super) facts: Vec<(&'static str, String)>,
    /// The words of the link to Monita: "Open the incident".
    pub(super) link_label: &'static str,
}

/// Text cut to `max` characters, for apps that refuse longer fields.
fn cut(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

/// A Discord webhook body: one embed whose title links to Monita. Mentions in the text never
/// ping, and the post shows as Monita whatever the webhook is named.
pub(super) fn discord(card: &Card, link: &str) -> Value {
    let mut fields = vec![
        json!({ "name": "Project", "value": cut(&no_links(&card.project), 1024), "inline": true }),
    ];
    for (name, value) in &card.facts {
        fields.push(json!({ "name": name, "value": cut(&no_links(value), 1024), "inline": value.chars().count() <= 40 }));
    }
    json!({
        "username": "Monita",
        "allowed_mentions": { "parse": [] },
        "embeds": [{
            "title": cut(&format!("{} {}: {}", card.tone.mark(), card.label, card.title), 256),
            "url": link,
            "color": card.tone.colour(),
            "fields": fields,
        }],
    })
}

/// Discord turns `[words](url)` in a field into a link that hides its address, so a ping's cause or
/// a reporter's title could plant one. Escaped brackets show as typed.
fn no_links(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('[', "\\[")
        .replace(']', "\\]")
}

/// Telegram's HTML needs these three escaped in text and in a link.
fn html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// A Telegram `sendMessage` body for `chat`, in HTML: a bold title, the project, the facts and the
/// link. The link shows its own URL: Telegram drops links to hosts it will not open, such as
/// `localhost`, and a named link would then leave nothing to copy.
pub(super) fn telegram(card: &Card, link: &str, chat: &str) -> Value {
    let mut text = format!(
        "{} <b>{}: {}</b>\n{}\n",
        card.tone.mark(),
        card.label,
        html(&card.title),
        html(&card.project)
    );
    for (name, value) in &card.facts {
        text.push_str(&format!("\n<b>{name}:</b> {}", html(value)));
    }
    let link = html(link);
    text.push_str(&format!(
        "\n\n{}: <a href=\"{link}\">{link}</a>",
        card.link_label
    ));
    json!({
        "chat_id": chat,
        "text": cut(&text, 4096),
        "parse_mode": "HTML",
        "disable_web_page_preview": true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card() -> Card {
        Card {
            tone: Tone::Critical,
            label: "Critical",
            title: "API <health> is down".into(),
            project: "Pay & Go".into(),
            facts: vec![("Cause", "HTTP 503 @everyone".into())],
            link_label: "Open the incident",
        }
    }

    #[test]
    fn discord_gets_one_embed_that_links_to_monita_and_pings_nobody() {
        let b = discord(&card(), "https://m.co/i/7");
        assert_eq!(b["username"], "Monita");
        assert_eq!(b["allowed_mentions"]["parse"], json!([]));
        let e = &b["embeds"][0];
        assert_eq!(e["title"], "🔴 Critical: API <health> is down");
        assert_eq!(e["url"], "https://m.co/i/7");
        assert_eq!(e["color"], 0xE5484D);
        assert_eq!(
            e["fields"][0],
            json!({ "name": "Project", "value": "Pay & Go", "inline": true })
        );
        assert_eq!(e["fields"][1]["name"], "Cause");
    }

    #[test]
    fn a_discord_field_cannot_hide_a_link_behind_words() {
        let mut c = card();
        c.facts = vec![("Cause", r"[Sign in again](https://evil.test) \o/".into())];
        let b = discord(&c, "https://m.co/i/7");
        assert_eq!(
            b["embeds"][0]["fields"][1]["value"],
            r"\[Sign in again\](https://evil.test) \\o/"
        );
    }

    #[test]
    fn telegram_html_escapes_every_word_from_outside() {
        let b = telegram(&card(), "https://m.co/i/7?a=1&b=\"2\"", "-100");
        assert_eq!(b["chat_id"], "-100");
        assert_eq!(b["parse_mode"], "HTML");
        assert_eq!(
            b["text"],
            "🔴 <b>Critical: API &lt;health&gt; is down</b>\nPay &amp; Go\n\n<b>Cause:</b> HTTP 503 @everyone\n\nOpen the incident: <a href=\"https://m.co/i/7?a=1&amp;b=&quot;2&quot;\">https://m.co/i/7?a=1&amp;b=&quot;2&quot;</a>"
        );
    }
}
