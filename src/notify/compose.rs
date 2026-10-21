//! What an alert says: the email subject and text, and the JSON body, built once per event and
//! then shaped per webhook format by `format`.

use super::card::{Card, Tone};
use crate::{
    incidents::{Ended, Event},
    platform::{now, time},
};

pub(super) struct Composed {
    pub(super) subject: String,
    pub(super) text: String,
    pub(super) json: serde_json::Value,
    /// The Monita page the alert is about.
    pub(super) link: String,
    /// Names the incident or the source, so a resolve finds its trigger (PagerDuty's dedup key).
    pub(super) dedup: String,
    /// True when the alert ends something: an incident closed, or a source back up.
    pub(super) resolves: bool,
    /// The same alert laid out for Discord and Telegram.
    pub(super) card: Card,
}

/// "Critical" or "Warning".
fn severity_word(severity: &str) -> &'static str {
    if severity == "critical" {
        "Critical"
    } else {
        "Warning"
    }
}

fn open_source(kind: crate::incidents::SourceKind) -> &'static str {
    match kind {
        crate::incidents::SourceKind::Monitor => "Open the monitor",
        crate::incidents::SourceKind::Heartbeat => "Open the heartbeat",
    }
}

pub(super) fn compose(
    ev: &Event,
    project: &(String, String),
    base_url: &str,
) -> (String, Composed) {
    let (name, slug) = project;
    match ev {
        Event::Opened {
            incident_id,
            title,
            severity,
            cause,
            source_name,
            ..
        } => {
            let url = format!("{base_url}/projects/{slug}/incidents/{incident_id}");
            let sev = severity.to_uppercase();
            let text = format!("[{sev}] {name} / {title}: {cause}");
            (
                severity.clone(),
                Composed {
                    card: Card {
                        tone: Tone::of(severity),
                        label: severity_word(severity),
                        title: title.clone(),
                        project: name.clone(),
                        facts: vec![("Cause", cause.clone())],
                        link_label: "Open the incident",
                    },
                    link: url.clone(),
                    dedup: format!("incident-{incident_id}"),
                    resolves: false,
                    subject: format!("[{sev}] {name} / {title}"),
                    json: serde_json::json!({
                        "text": text, "event": "incident.opened", "project": slug, "source": source_name,
                        "severity": severity, "cause": cause, "incident_url": url, "at": now(),
                    }),
                    text: format!("{text}\n\nOpen the incident: {url}\n"),
                },
            )
        }
        Event::Resolved {
            incident_id,
            severity,
            duration_s,
            source_name,
            ended,
            ..
        } => {
            let url = format!("{base_url}/projects/{slug}/incidents/{incident_id}");
            let how = match ended {
                Ended::Recovered => "is up",
                Ended::Deleted => "was deleted",
                Ended::ByHand => "was marked resolved by a tech lead",
            };
            let text = format!(
                "[RESOLVED] {name} / {source_name} {how} after {}",
                time::duration(*duration_s)
            );
            (
                severity.clone(),
                Composed {
                    card: Card {
                        tone: Tone::Good,
                        label: "Resolved",
                        title: format!("{source_name} {how}"),
                        project: name.clone(),
                        facts: vec![("Down for", time::duration(*duration_s))],
                        link_label: "Open the incident",
                    },
                    link: url.clone(),
                    dedup: format!("incident-{incident_id}"),
                    resolves: true,
                    subject: text.clone(),
                    json: serde_json::json!({
                        "text": text, "event": "incident.resolved", "project": slug, "source": source_name,
                        "severity": severity, "duration_s": duration_s, "incident_url": url, "at": now(),
                    }),
                    text: format!("{text}\n\nTimeline: {url}\n"),
                },
            )
        }
        Event::Down {
            kind,
            source_id,
            source_name,
            title,
            severity,
            cause,
            ..
        } => {
            let url = source_url(base_url, slug, *kind, *source_id);
            let text = format!("[DOWN] {name} / {title}: {cause}");
            (
                severity.clone(),
                Composed {
                    card: Card {
                        tone: Tone::of(severity),
                        label: "Down",
                        title: title.clone(),
                        project: name.clone(),
                        facts: vec![("Cause", cause.clone()), ("Incident", "None opened".into())],
                        link_label: open_source(*kind),
                    },
                    link: url.clone(),
                    dedup: format!("{}-{source_id}", kind_word(*kind)),
                    resolves: false,
                    subject: format!("[DOWN] {name} / {title}"),
                    json: serde_json::json!({
                        "text": text, "event": "source.down", "project": slug, "source": source_name,
                        "severity": severity, "cause": cause, "source_url": url, "at": now(),
                    }),
                    text: format!(
                        "{text}\n\nNo incident was opened. Report one if users should know.\n{url}\n"
                    ),
                },
            )
        }
        Event::Up {
            kind,
            source_id,
            source_name,
            severity,
            duration_s,
            ..
        } => {
            let url = source_url(base_url, slug, *kind, *source_id);
            let text = format!(
                "[UP] {name} / {source_name} is up after {}",
                time::duration(*duration_s)
            );
            (
                severity.clone(),
                Composed {
                    card: Card {
                        tone: Tone::Good,
                        label: "Up",
                        title: format!("{source_name} is up"),
                        project: name.clone(),
                        facts: vec![("Down for", time::duration(*duration_s))],
                        link_label: open_source(*kind),
                    },
                    link: url.clone(),
                    dedup: format!("{}-{source_id}", kind_word(*kind)),
                    resolves: true,
                    subject: text.clone(),
                    json: serde_json::json!({
                        "text": text, "event": "source.up", "project": slug, "source": source_name,
                        "severity": severity, "duration_s": duration_s, "source_url": url, "at": now(),
                    }),
                    text: format!("{text}\n\n{url}\n"),
                },
            )
        }
    }
}

/// A certificate that expires soon or has expired (v1.1 Q25). Always the warning route.
pub(super) fn cert_warning(
    w: &super::CertWarning<'_>,
    project: &(String, String),
    base_url: &str,
    at: i64,
) -> Composed {
    let (name, slug) = project;
    let url = source_url(
        base_url,
        slug,
        crate::incidents::SourceKind::Monitor,
        w.monitor_id,
    );
    let day = time::day(w.expires_at);
    let when = if w.expires_at <= at {
        format!("expired on {day}")
    } else {
        let days = (w.expires_at - at) / 86_400;
        match days {
            0 => format!("expires today, on {day}"),
            1 => format!("expires in 1 day, on {day}"),
            n => format!("expires in {n} days, on {day}"),
        }
    };
    let text = format!("[CERT] {name} / {}: the certificate {when}", w.monitor_name);
    Composed {
        card: Card {
            tone: Tone::Warning,
            label: "Certificate",
            title: format!("{}: the certificate {when}", w.monitor_name),
            project: name.clone(),
            facts: Vec::new(),
            link_label: "Open the monitor",
        },
        link: url.clone(),
        dedup: format!("cert:{}:{}", w.monitor_id, w.expires_at),
        resolves: false,
        subject: text.clone(),
        json: serde_json::json!({
            "text": text, "event": "certificate.expiring", "project": slug, "source": w.monitor_name,
            "severity": "warning", "expires_at": w.expires_at, "source_url": url, "at": at,
        }),
        text: format!("{text}\n\nRenew it before then. No incident was opened.\n{url}\n"),
    }
}

/// The page of a monitor or a heartbeat, for an alert with no incident to link.
fn source_url(base_url: &str, slug: &str, kind: crate::incidents::SourceKind, id: i64) -> String {
    format!("{base_url}/projects/{slug}/{}s/{id}", kind_word(kind))
}

fn kind_word(kind: crate::incidents::SourceKind) -> &'static str {
    match kind {
        crate::incidents::SourceKind::Monitor => "monitor",
        crate::incidents::SourceKind::Heartbeat => "heartbeat",
    }
}

/// A reporter's words for a chat's `text` field: Slack reads `<...>` as a link or a mention such
/// as `<!channel>`, so an anonymous title must not carry one into the team's channel. The
/// `title` field keeps the raw words for code that reads the JSON.
pub(super) fn chat_safe(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// The webhook body for a new ticket. Tickets send no email.
pub(super) fn ticket_json(
    name: &str,
    slug: &str,
    number: i64,
    title: &str,
    base_url: &str,
    at: i64,
) -> serde_json::Value {
    serde_json::json!({
        "text": format!("[TICKET] {name} #{number}: {}", chat_safe(title)),
        "event": "ticket.created",
        "project": slug,
        "ticket_number": number,
        "title": title,
        "ticket_url": format!("{base_url}/projects/{slug}/tickets/{number}"),
        "at": at,
    })
}

/// One card's test alert. It names the card, so a team with several cards sees which one spoke.
pub(super) fn test_alert(
    name: &str,
    slug: &str,
    card_name: &str,
    base_url: &str,
    at: i64,
) -> Composed {
    let link = format!("{base_url}/projects/{slug}/settings/alerts");
    let text = format!("[TEST] {name}: the alert card \"{card_name}\" reaches this channel.");
    Composed {
        card: Card {
            tone: Tone::Info,
            label: "Test",
            title: format!("{card_name} reaches this chat"),
            project: name.to_string(),
            facts: Vec::new(),
            link_label: "Open the alert settings",
        },
        subject: text.clone(),
        text: format!("{text}\n\n{link}\n"),
        json: serde_json::json!({ "text": text, "event": "test", "project": slug, "card": card_name, "at": at }),
        link,
        dedup: "test".into(),
        resolves: false,
    }
}
