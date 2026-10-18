//! notify module: alert routes, the outbox, delivery.
//! Spec: docs/technical-specs/05-module-definitions.md section 5.5, api-specs 06.
//! Callers queue messages inside their own write transaction; the outbox loop delivers them,
//! so a slow SMTP server or webhook never blocks a check or a request.

pub mod api;
mod delivery;
mod digest;
mod mailer;
mod preview;
mod store;

use turso::Connection;

use crate::{
    incidents::Event,
    platform::{db::Db, now, time},
};

pub use delivery::{deliver_due, spawn_delivery};
pub use digest::digest;
pub use mailer::{Mailer, SentEmail};
pub use preview::{Preview, previews};
pub use store::Route;

/// AC-15.02: a new project's two routes. Critical calls its webhook once one is set; warning does not.
pub async fn create_routes(c: &Connection, project_id: i64) -> anyhow::Result<()> {
    store::create_routes(c, project_id).await
}

/// Retention (US-32): finished rows older than `cutoff`. Runs inside the caller's write.
/// Deletes up to `limit` rows; returns how many. The caller loops until it gets fewer.
pub async fn purge_delivered(c: &Connection, cutoff: i64, limit: i64) -> anyhow::Result<u64> {
    store::purge_delivered(c, cutoff, limit).await
}

/// Queues one email. Call it inside `Db::write` with the change that caused it.
pub async fn queue_email(
    c: &Connection,
    to: &str,
    subject: &str,
    body: &str,
    at: i64,
) -> anyhow::Result<()> {
    store::queue(c, "email", to, Some(subject), body, at).await
}

#[derive(Debug, thiserror::Error)]
pub enum RouteError {
    #[error("Not a valid email address: {0}")]
    BadEmail(String),
    #[error("The webhook URL must start with http:// or https://.")]
    BadWebhook,
    #[error(
        "A route takes {} addresses at most.",
        crate::platform::caps::EMAILS_PER_ROUTE
    )]
    TooManyEmails,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub async fn routes(db: &Db, project_id: i64) -> anyhow::Result<Vec<Route>> {
    store::routes(&db.read().await?, project_id).await
}

/// US-15. One email per line; an empty webhook URL turns the webhook off.
pub async fn save_route(
    db: &Db,
    project_id: i64,
    severity: &str,
    emails_raw: &str,
    webhook_raw: &str,
    use_webhook: bool,
) -> Result<(), RouteError> {
    if !matches!(severity, "critical" | "warning") {
        return Err(RouteError::Other(anyhow::anyhow!(
            "unknown severity {severity}"
        )));
    }
    let mut emails = Vec::new();
    for line in emails_raw.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let email = crate::accounts::normalize_email(line)
            .map_err(|_| RouteError::BadEmail(line.to_string()))?;
        if !emails.contains(&email) {
            emails.push(email);
        }
    }
    if emails.len() > crate::platform::caps::EMAILS_PER_ROUTE {
        return Err(RouteError::TooManyEmails);
    }
    let webhook = webhook_raw.trim();
    if !webhook.is_empty() && !crate::platform::valid_http_url(webhook) {
        return Err(RouteError::BadWebhook);
    }
    let webhook = (!webhook.is_empty()).then_some(webhook);
    db.write(async |c| {
        store::save_route(
            c,
            project_id,
            severity,
            &emails.join("\n"),
            webhook,
            use_webhook,
        )
        .await
    })
    .await?;
    Ok(())
}

pub(super) struct Composed {
    subject: String,
    text: String,
    json: serde_json::Value,
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
            deleted,
            ..
        } => {
            let url = format!("{base_url}/projects/{slug}/incidents/{incident_id}");
            let how = if *deleted { "was deleted" } else { "is up" };
            let text = format!(
                "[RESOLVED] {name} / {source_name} {how} after {}",
                time::duration(*duration_s)
            );
            (
                severity.clone(),
                Composed {
                    subject: text.clone(),
                    json: serde_json::json!({
                        "text": text, "event": "incident.resolved", "project": slug, "source": source_name,
                        "severity": severity, "duration_s": duration_s, "incident_url": url, "at": now(),
                    }),
                    text: format!("{text}\n\nTimeline: {url}\n"),
                },
            )
        }
    }
}

/// US-16. Queues one email per address and one webhook call for the event's severity.
/// Call it inside the same write transaction as `incidents::observe`.
pub async fn send(c: &Connection, ev: &Event, base_url: &str) -> anyhow::Result<()> {
    let project_id = match ev {
        Event::Opened { project_id, .. } | Event::Resolved { project_id, .. } => *project_id,
    };
    let Some(project) = crate::projects::name_slug(c, project_id).await? else {
        return Ok(());
    };
    let (severity, msg) = compose(ev, &project, base_url);
    let Some(route) = store::route(c, project_id, &severity).await? else {
        return Ok(());
    };
    let at = now();
    for email in &route.emails {
        store::queue(c, "email", email, Some(&msg.subject), &msg.text, at).await?;
    }
    if route.use_webhook
        && let Some(url) = &route.webhook_url
    {
        store::queue(c, "webhook", url, None, &msg.json.to_string(), at).await?;
    }
    Ok(())
}

/// AC-30.03: one webhook call per new ticket, on the warning route, when its webhook is on.
pub async fn send_ticket(
    c: &Connection,
    project_id: i64,
    number: i64,
    title: &str,
    base_url: &str,
) -> anyhow::Result<()> {
    let Some(route) = store::route(c, project_id, "warning").await? else {
        return Ok(());
    };
    let (true, Some(url)) = (route.use_webhook, route.webhook_url.as_deref()) else {
        return Ok(());
    };
    let Some((name, slug)) = crate::projects::name_slug(c, project_id).await? else {
        return Ok(());
    };
    let body = ticket_json(&name, &slug, number, title, base_url, now());
    store::queue(c, "webhook", url, None, &body.to_string(), now()).await
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
        "text": format!("[TICKET] {name} #{number}: {title}"),
        "event": "ticket.created",
        "project": slug,
        "ticket_number": number,
        "title": title,
        "ticket_url": format!("{base_url}/projects/{slug}/tickets/{number}"),
        "at": at,
    })
}

/// The test alert: its text, email body and webhook body.
pub(super) fn test_message(
    name: &str,
    slug: &str,
    severity: &str,
    base_url: &str,
    at: i64,
) -> (String, String, serde_json::Value) {
    let text = format!("[TEST] {name}: alerts for {severity} reach this channel.");
    let body = format!("{text}\n\n{base_url}/projects/{slug}/settings/alerts\n");
    let json = serde_json::json!({ "text": text, "event": "test", "project": slug, "severity": severity, "at": at });
    (text, body, json)
}

/// The test button on the alerts page: one message per recipient of that severity.
pub async fn send_test(
    db: &Db,
    project_id: i64,
    severity: &str,
    base_url: &str,
) -> anyhow::Result<usize> {
    db.write(async |c| {
        let Some((name, slug)) = crate::projects::name_slug(c, project_id).await? else {
            return Ok(0);
        };
        let Some(route) = store::route(c, project_id, severity).await? else {
            return Ok(0);
        };
        let at = now();
        let (text, body, json) = test_message(&name, &slug, severity, base_url, at);
        let mut n = 0;
        for email in &route.emails {
            store::queue(c, "email", email, Some(&text), &body, at).await?;
            n += 1;
        }
        if let Some(url) = route.webhook_url.as_deref().filter(|_| route.use_webhook) {
            store::queue(c, "webhook", url, None, &json.to_string(), at).await?;
            n += 1;
        }
        Ok(n)
    })
    .await
}

/// How email delivery went since `since`, for the system page.
#[derive(Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct EmailHealth {
    pub sent: i64,
    pub failed: i64,
    pub waiting: i64,
    pub last_sent_at: Option<i64>,
    /// The newest delivery error and when it happened.
    pub last_error: Option<(String, i64)>,
}

pub async fn email_health(db: &Db, since: i64) -> anyhow::Result<EmailHealth> {
    let c = db.read().await?;
    let (sent, failed, waiting) = store::email_counts(&c, since).await?;
    let (last_sent_at, last_error) = store::email_latest(&c).await?;
    Ok(EmailHealth {
        sent,
        failed,
        waiting,
        last_sent_at,
        last_error,
    })
}

/// Outbox rows not yet sent or failed, retries included. `/healthz` shows it.
pub async fn pending_count(c: &Connection) -> anyhow::Result<i64> {
    store::pending_count(c).await
}

/// Read-only: is any outbox row due at `at`? The scheduler asks before it spawns a delivery pass.
pub async fn any_due(c: &Connection, at: i64) -> anyhow::Result<bool> {
    store::any_due(c, at).await
}
