//! Sample alerts for the Alerts page, built by the same code that builds real ones.

use serde::Serialize;

use super::{compose, test_message, ticket_json, tickets::Moved, tickets::ticket_moved_json};
use crate::incidents::Event;

/// One kind of alert as its recipients get it.
#[derive(Serialize)]
pub struct Preview {
    /// The webhook `event` field: `incident.opened`, `incident.resolved`, `ticket.created`,
    /// `ticket.moved`, `test`.
    pub event: &'static str,
    pub label: &'static str,
    /// The settings that send it: `critical`, `warning` or `ticket`.
    pub route: &'static str,
    /// `None` when this kind sends no email.
    pub email: Option<EmailPreview>,
    pub webhook: serde_json::Value,
}

#[derive(Serialize)]
pub struct EmailPreview {
    pub subject: String,
    pub body: String,
}

/// Every kind of alert for this project, with sample values.
pub fn previews(name: &str, slug: &str, base_url: &str, at: i64) -> Vec<Preview> {
    let project = (name.to_string(), slug.to_string());
    let opened = Event::Opened {
        incident_id: 42,
        project_id: 0,
        source_name: "Checkout API".into(),
        title: "Checkout API is down".into(),
        severity: "critical".into(),
        cause: "HTTP 503 on 2 checks in a row".into(),
    };
    let resolved = Event::Resolved {
        incident_id: 42,
        project_id: 0,
        source_name: "Checkout API".into(),
        severity: "critical".into(),
        duration_s: 754,
        ended: crate::incidents::Ended::Recovered,
    };
    let mut out = Vec::new();
    for (event, label, ev) in [
        ("incident.opened", "Incident opened", opened),
        ("incident.resolved", "Incident resolved", resolved),
    ] {
        let (_, msg) = compose(&ev, &project, base_url);
        let mut webhook = msg.json;
        webhook["at"] = at.into();
        out.push(Preview {
            event,
            label,
            route: "critical",
            email: Some(EmailPreview {
                subject: msg.subject,
                body: msg.text,
            }),
            webhook,
        });
    }
    out.push(Preview {
        event: "ticket.created",
        label: "New ticket",
        route: "ticket",
        email: None,
        webhook: ticket_json(
            name,
            slug,
            17,
            "Export to Excel drops the last row",
            base_url,
            at,
        ),
    });
    let moved = Moved {
        number: 17,
        title: "Export to Excel drops the last row",
        from: "in_progress",
        to: "done",
        close_reason: Some("fixed"),
    };
    out.push(Preview {
        event: "ticket.moved",
        label: "Ticket moved",
        route: "ticket",
        email: None,
        webhook: ticket_moved_json(name, slug, &moved, base_url, at),
    });
    let (subject, body, webhook) = test_message(name, slug, "critical", base_url, at);
    out.push(Preview {
        event: "test",
        label: "Test",
        route: "critical",
        email: Some(EmailPreview { subject, body }),
        webhook,
    });
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn previews_cover_every_alert_kind_as_real_alerts_build_them() {
        let all = super::previews("Payments", "payments-abc123", "https://m.co", 7);
        let events: Vec<&str> = all.iter().map(|p| p.event).collect();
        assert_eq!(
            events,
            [
                "incident.opened",
                "incident.resolved",
                "ticket.created",
                "ticket.moved",
                "test"
            ]
        );
        let opened = &all[0];
        assert_eq!(opened.webhook["event"], "incident.opened");
        assert_eq!(opened.webhook["project"], "payments-abc123");
        assert_eq!(
            opened.webhook["incident_url"],
            "https://m.co/projects/payments-abc123/incidents/42"
        );
        assert_eq!(opened.webhook["at"], 7);
        assert_eq!(
            opened.email.as_ref().unwrap().subject,
            "[CRITICAL] Payments / Checkout API is down"
        );
        assert!(all[2].email.is_none(), "a ticket sends no email");
        assert_eq!(all[2].route, "ticket");
        assert_eq!(
            all[3].webhook["text"],
            "[TICKET] Payments #17 moved from In progress to Done (Fixed): Export to Excel drops the last row"
        );
        assert!(
            all[4]
                .email
                .as_ref()
                .unwrap()
                .body
                .contains("/projects/payments-abc123/settings/alerts")
        );
    }
}
