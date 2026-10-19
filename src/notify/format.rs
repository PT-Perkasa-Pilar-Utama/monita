//! How a severity's webhook speaks (v1.1 Q17): where Monita posts and the JSON it sends, per
//! format. Every format is one JSON POST, so the outbox delivers them all the same way.

use serde_json::{Value, json};

use super::{RouteError, compose::Composed};

pub(super) const FORMATS: [&str; 5] = ["json", "discord", "telegram", "ntfy", "pagerduty"];

/// PagerDuty's Events API v2. A route in this format posts here, whatever URL the form sent.
pub(super) const PAGERDUTY_URL: &str = "https://events.pagerduty.com/v2/enqueue";

/// A route's webhook as it is stored: the URL to post to and the key, checked for the format.
/// An empty URL turns the webhook off, except for PagerDuty, whose URL is fixed.
pub(super) fn check(
    format: &str,
    url: &str,
    key: &str,
) -> Result<(Option<String>, Option<String>), RouteError> {
    let (url, key) = (url.trim(), key.trim());
    if !FORMATS.contains(&format) {
        return Err(RouteError::BadFormat);
    }
    let url = match format {
        "pagerduty" => Some(PAGERDUTY_URL.to_string()),
        _ if url.is_empty() => None,
        _ if !crate::platform::valid_http_url(url) => return Err(RouteError::BadWebhook),
        "ntfy" if ntfy_topic(url).is_none() => return Err(RouteError::NtfyTopic),
        _ => Some(url.to_string()),
    };
    let key = match format {
        "telegram" if !telegram_chat(key) => {
            return Err(RouteError::BadKey(
                "Enter the chat ID, such as -1001234567890 or @team_alerts.",
            ));
        }
        "pagerduty"
            if key.is_empty()
                || key.len() > 64
                || !key.chars().all(|c| c.is_ascii_alphanumeric()) =>
        {
            return Err(RouteError::BadKey(
                "Enter the routing key of the PagerDuty integration.",
            ));
        }
        "telegram" | "pagerduty" => Some(key.to_string()),
        _ => None,
    };
    Ok((url, key))
}

/// A numeric chat ID, or a public channel's `@name`.
fn telegram_chat(key: &str) -> bool {
    let digits = key.strip_prefix('-').unwrap_or(key);
    let numeric =
        !digits.is_empty() && digits.len() <= 20 && digits.chars().all(|c| c.is_ascii_digit());
    let channel = key.strip_prefix('@').is_some_and(|n| {
        (5..=32).contains(&n.len()) && n.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    });
    numeric || channel
}

/// `https://ntfy.sh/team-alerts` as the server's root and the topic. ntfy reads a JSON body only
/// when it is posted to the root, with the topic inside.
fn ntfy_topic(url: &str) -> Option<(String, String)> {
    let (root, topic) = url.trim_end_matches('/').rsplit_once('/')?;
    let scheme_only = root.ends_with(':') || root.ends_with("/");
    (!topic.is_empty() && !scheme_only && !topic.contains(['?', '#']))
        .then(|| (format!("{root}/"), topic.to_string()))
}

/// Text cut to `max` characters, for channels that refuse longer messages.
fn cut(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

/// Where to post one alert and the body, for the route's format. `url` and `key` are as `check`
/// stored them.
pub(super) fn webhook(
    format: &str,
    url: &str,
    key: Option<&str>,
    slug: &str,
    msg: &Composed,
) -> (String, Value) {
    let line = msg.json["text"].as_str().unwrap_or(&msg.subject);
    let critical = msg.json["severity"] == "critical";
    match format {
        "discord" => (
            url.to_string(),
            // No @everyone or role pings from text that a cause or a name may carry.
            json!({ "content": cut(&format!("{line}\n{}", msg.link), 2000), "allowed_mentions": { "parse": [] } }),
        ),
        "telegram" => (
            url.to_string(),
            json!({ "chat_id": key.unwrap_or_default(), "text": cut(&format!("{line}\n{}", msg.link), 4096), "disable_web_page_preview": true }),
        ),
        "ntfy" => {
            let (root, topic) = ntfy_topic(url).unwrap_or_else(|| (url.to_string(), String::new()));
            let priority = match (msg.resolves, critical) {
                (true, _) => 3,
                (false, true) => 5,
                (false, false) => 4,
            };
            (
                root,
                json!({ "topic": topic, "title": cut(&msg.subject, 250), "message": cut(line, 4000), "priority": priority, "click": msg.link }),
            )
        }
        "pagerduty" => {
            let dedup_key = format!("monita-{slug}-{}", msg.dedup);
            let body = if msg.resolves {
                json!({ "routing_key": key.unwrap_or_default(), "event_action": "resolve", "dedup_key": dedup_key })
            } else {
                json!({
                    "routing_key": key.unwrap_or_default(), "event_action": "trigger", "dedup_key": dedup_key,
                    "payload": { "summary": cut(line, 1024), "source": format!("monita/{slug}"),
                                 "severity": if critical { "critical" } else { "warning" }, "custom_details": msg.json },
                    "links": [{ "href": msg.link, "text": "Open in Monita" }],
                })
            };
            (PAGERDUTY_URL.to_string(), body)
        }
        _ => (url.to_string(), msg.json.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(resolves: bool) -> Composed {
        Composed {
            subject: "[CRITICAL] Pay / API is down".into(),
            text: "email text".into(),
            json: json!({ "text": "[CRITICAL] Pay / API is down: HTTP 503", "severity": "critical" }),
            link: "https://m.co/projects/pay/incidents/7".into(),
            dedup: "incident-7".into(),
            resolves,
        }
    }

    #[test]
    fn each_format_checks_the_url_and_the_key_it_needs() {
        let ok = |f: &str, u: &str, k: &str| check(f, u, k).map_err(|e| e.to_string());
        assert_eq!(
            ok("json", "https://h.co/x", "ignored"),
            Ok((Some("https://h.co/x".into()), None))
        );
        assert_eq!(
            ok("json", "", ""),
            Ok((None, None)),
            "an empty URL turns the webhook off"
        );
        assert!(ok("smoke", "https://h.co", "").is_err());
        assert!(
            ok(
                "telegram",
                "https://api.telegram.org/bot1:x/sendMessage",
                ""
            )
            .is_err()
        );
        assert_eq!(
            ok(
                "telegram",
                "https://api.telegram.org/bot1:x/sendMessage",
                "-1001234"
            )
            .unwrap()
            .1
            .as_deref(),
            Some("-1001234")
        );
        assert!(ok("telegram", "https://t.co/x", "@team_alerts").is_ok());
        assert!(ok("telegram", "https://t.co/x", "chat 1").is_err());
        assert_eq!(
            ok("pagerduty", "", "R0ut1ngKey").unwrap().0.as_deref(),
            Some(PAGERDUTY_URL),
            "the URL is fixed"
        );
        assert!(ok("pagerduty", "", "").is_err());
        assert!(
            ok("ntfy", "https://ntfy.sh/", "").is_err(),
            "a topic is needed"
        );
        assert!(ok("ntfy", "https://ntfy.sh/team-alerts", "").is_ok());
    }

    #[test]
    fn chat_formats_carry_the_alert_line_and_the_link() {
        let (to, body) = webhook(
            "discord",
            "https://discord.com/api/webhooks/1/x",
            None,
            "pay",
            &msg(false),
        );
        assert_eq!(to, "https://discord.com/api/webhooks/1/x");
        assert_eq!(
            body["content"],
            "[CRITICAL] Pay / API is down: HTTP 503\nhttps://m.co/projects/pay/incidents/7"
        );
        assert_eq!(
            body["allowed_mentions"]["parse"],
            json!([]),
            "no @everyone from alert text"
        );
        let (_, body) = webhook(
            "telegram",
            "https://t.co/bot/sendMessage",
            Some("-100"),
            "pay",
            &msg(false),
        );
        assert_eq!(body["chat_id"], "-100");
        let (to, body) = webhook(
            "ntfy",
            "https://ntfy.sh/team-alerts",
            None,
            "pay",
            &msg(false),
        );
        assert_eq!(to, "https://ntfy.sh/");
        assert_eq!(
            (body["topic"].as_str(), body["priority"].as_i64()),
            (Some("team-alerts"), Some(5))
        );
        let (_, body) = webhook(
            "ntfy",
            "https://ntfy.sh/team-alerts",
            None,
            "pay",
            &msg(true),
        );
        assert_eq!(body["priority"], 3, "a resolve is not urgent");
        let (_, body) = webhook("json", "https://h.co", None, "pay", &msg(false));
        assert_eq!(body, msg(false).json, "json is the body Monita always sent");
    }

    #[test]
    fn pagerduty_resolves_the_event_it_triggered() {
        let (to, open) = webhook("pagerduty", PAGERDUTY_URL, Some("KEY"), "pay", &msg(false));
        let (_, close) = webhook("pagerduty", PAGERDUTY_URL, Some("KEY"), "pay", &msg(true));
        assert_eq!(to, PAGERDUTY_URL);
        assert_eq!(open["event_action"], "trigger");
        assert_eq!(open["payload"]["severity"], "critical");
        assert_eq!(close["event_action"], "resolve");
        assert_eq!(open["dedup_key"], "monita-pay-incident-7");
        assert_eq!(open["dedup_key"], close["dedup_key"]);
        assert_eq!(close["routing_key"], "KEY");
    }
}
