//! notify module: alert routes, the outbox, delivery.
//! Spec: docs/technical-specs/05-module-definitions.md section 5.5, api-specs 06.
//! Callers queue messages inside their own write transaction; the outbox loop delivers them,
//! so a slow SMTP server or webhook never blocks a check or a request.

pub mod api;
mod compose;
mod delivery;
mod digest;
mod format;
mod mailer;
mod preview;
mod store;
mod tickets;

use turso::Connection;

use compose::{cert_warning, chat_safe, compose, test_message, ticket_json};

use crate::{
    incidents::Event,
    platform::{db::Db, now},
};

pub use delivery::{deliver_due, spawn_delivery};
pub use digest::digest;
pub use mailer::{Mailer, SentEmail, test_connection};
pub use preview::{Preview, previews};
pub use store::{Route, TicketAlerts};
pub use tickets::{
    Moved, save_ticket_alerts, send_ticket, send_ticket_moved, send_ticket_test, ticket_alerts,
};

/// AC-15.02: a new project's two routes, both emailing the owner, with the webhook unticked until
/// the owner adds one (v1.1 Q27).
pub async fn create_routes(c: &Connection, project_id: i64, owner_id: i64) -> anyhow::Result<()> {
    let owner = crate::accounts::active_email(c, owner_id).await?;
    store::create_routes(c, project_id, owner.as_deref().unwrap_or_default()).await
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
    #[error("Pick JSON, Discord, Telegram, ntfy or PagerDuty.")]
    BadFormat,
    #[error("Enter the topic URL, such as https://ntfy.sh/team-alerts.")]
    NtfyTopic,
    /// The chat ID or routing key the format needs, with what to enter.
    #[error("{0}")]
    BadKey(&'static str),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// What a severity's form sends: who gets email, and the webhook in its format.
pub struct RouteForm<'a> {
    pub emails: &'a str,
    pub webhook_url: &'a str,
    pub use_webhook: bool,
    /// One of `format::FORMATS`; empty reads as `json`.
    pub webhook_format: &'a str,
    /// Telegram's chat ID or PagerDuty's routing key.
    pub webhook_key: &'a str,
}

pub async fn routes(db: &Db, project_id: i64) -> anyhow::Result<Vec<Route>> {
    store::routes(&db.read().await?, project_id).await
}

/// US-15. One email per line; an empty webhook URL turns the webhook off, except for PagerDuty,
/// whose URL is fixed (v1.1 Q17).
pub async fn save_route(
    db: &Db,
    project_id: i64,
    severity: &str,
    form: &RouteForm<'_>,
) -> Result<(), RouteError> {
    let emails_raw = form.emails;
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
    let webhook_format = Some(form.webhook_format.trim())
        .filter(|f| !f.is_empty())
        .unwrap_or("json");
    let (url, key) = format::check(webhook_format, form.webhook_url, form.webhook_key)?;
    let saved = store::Route {
        severity: severity.to_string(),
        emails,
        webhook_url: url,
        use_webhook: form.use_webhook,
        enabled: true,
        webhook_format: webhook_format.to_string(),
        webhook_key: key,
    };
    db.write(async |c| store::save_route(c, project_id, &saved).await)
        .await?;
    Ok(())
}

/// Turns `critical`, `warning` or `ticket` alerts on or off. False for an unknown name.
pub async fn set_enabled(db: &Db, project_id: i64, route: &str, on: bool) -> anyhow::Result<bool> {
    if !matches!(route, "critical" | "warning" | "ticket") {
        return Ok(false);
    }
    db.write(async |c| store::set_enabled(c, project_id, route, on).await)
        .await
}

/// US-16. Queues one email per address and one webhook call for the event's severity.
/// Call it inside the same write transaction as `incidents::observe`.
pub async fn send(c: &Connection, ev: &Event, base_url: &str) -> anyhow::Result<()> {
    let project_id = match ev {
        Event::Opened { project_id, .. }
        | Event::Resolved { project_id, .. }
        | Event::Down { project_id, .. }
        | Event::Up { project_id, .. } => *project_id,
    };
    let Some(project) = crate::projects::name_slug(c, project_id).await? else {
        return Ok(());
    };
    let (severity, msg) = compose(ev, &project, base_url);
    queue_alert(c, project_id, &project.1, &severity, &msg).await?;
    Ok(())
}

/// A monitor whose HTTPS certificate expires soon (v1.1 Q25).
pub struct CertWarning<'a> {
    pub project_id: i64,
    pub monitor_id: i64,
    pub monitor_name: &'a str,
    pub expires_at: i64,
}

/// Queues the warning route's email and webhook for a certificate that expires soon. Opens no
/// incident. False when nothing was queued: no project, or warning alerts are off.
pub async fn send_cert_warning(
    c: &Connection,
    w: &CertWarning<'_>,
    base_url: &str,
) -> anyhow::Result<bool> {
    let Some(project) = crate::projects::name_slug(c, w.project_id).await? else {
        return Ok(false);
    };
    let msg = cert_warning(w, &project, base_url, now());
    queue_alert(c, w.project_id, &project.1, "warning", &msg).await
}

/// One email per address of the severity's route, and its webhook call. False when the route is
/// off.
async fn queue_alert(
    c: &Connection,
    project_id: i64,
    slug: &str,
    severity: &str,
    msg: &compose::Composed,
) -> anyhow::Result<bool> {
    let Some(route) = store::route(c, project_id, severity)
        .await?
        .filter(|r| r.enabled)
    else {
        return Ok(false);
    };
    let at = now();
    for email in &route.emails {
        store::queue(c, "email", email, Some(&msg.subject), &msg.text, at).await?;
    }
    queue_webhook(c, &route, slug, msg, at).await?;
    Ok(true)
}

/// The route's webhook call for one alert, in the route's format, when the webhook is on.
async fn queue_webhook(
    c: &Connection,
    route: &Route,
    slug: &str,
    msg: &compose::Composed,
    at: i64,
) -> anyhow::Result<bool> {
    let Some(url) = route.webhook_url.as_deref().filter(|_| route.use_webhook) else {
        return Ok(false);
    };
    let key = route.webhook_key.as_deref();
    let (target, body) = format::webhook(&route.webhook_format, url, key, slug, msg);
    store::queue(c, "webhook", &target, None, &body.to_string(), at).await?;
    Ok(true)
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
        let Some(route) = store::route(c, project_id, severity)
            .await?
            .filter(|r| r.enabled)
        else {
            return Ok(0);
        };
        let at = now();
        let (text, body, json) = test_message(&name, &slug, severity, base_url, at);
        let mut n = 0;
        for email in &route.emails {
            store::queue(c, "email", email, Some(&text), &body, at).await?;
            n += 1;
        }
        let mut msg = compose::Composed {
            subject: text,
            text: body,
            json,
            link: format!("{base_url}/projects/{slug}/settings/alerts"),
            dedup: "test".into(),
            resolves: false,
        };
        if queue_webhook(c, &route, &slug, &msg, at).await? {
            n += 1;
            // A PagerDuty test opens an incident there; resolve it at once.
            if route.webhook_format == "pagerduty" {
                msg.resolves = true;
                queue_webhook(c, &route, &slug, &msg, at).await?;
            }
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
