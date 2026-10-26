//! notify module: alert cards for incidents and tickets, the outbox, delivery.
//! Spec: docs/technical-specs/05-module-definitions.md section 5.5, api-specs 06.
//! Callers queue messages inside their own write transaction; the outbox loop delivers them,
//! so a slow SMTP server or webhook never blocks a check or a request.

pub mod api;
mod card;
pub mod cards;
mod compose;
mod delivery;
mod digest;
mod format;
mod mailer;
mod preview;
mod store;
mod telegram;
mod tickets;

use turso::Connection;

use compose::{cert_warning, chat_safe, compose, ticket_json};

use crate::{
    incidents::Event,
    platform::{db::Db, now},
};

use cards::Listen;
pub use cards::{AlertCard, CardForm};
pub use delivery::{deliver_due, spawn_delivery};
pub use digest::digest;
pub use mailer::{Mailer, SentEmail, test_connection};
pub use preview::{Preview, previews};
pub use tickets::{Moved, send_ticket, send_ticket_moved};

/// AC-15.02: a new project's alerts: the owner's email cards (v1.1 Q34, Q37). Runs inside the
/// caller's write.
pub async fn create_alerts(c: &Connection, project_id: i64) -> anyhow::Result<()> {
    cards::create_owner_cards(c, project_id).await
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

/// Why an alert card was refused.
#[derive(Debug, thiserror::Error)]
pub enum AlertError {
    #[error("Not a valid email address: {0}")]
    BadEmail(String),
    #[error("Enter at least one email address.")]
    NoEmails,
    #[error(
        "A card takes {} addresses at most.",
        crate::platform::caps::EMAILS_PER_CARD
    )]
    TooManyEmails,
    #[error("The webhook URL must start with http:// or https://.")]
    BadWebhook,
    #[error("Pick Email, JSON, Discord, Telegram, ntfy or PagerDuty.")]
    BadFormat,
    #[error("Enter the topic URL, such as https://ntfy.sh/team-alerts.")]
    NtfyTopic,
    /// The chat ID or routing key the format needs, with what to enter.
    #[error("{0}")]
    BadKey(&'static str),
    #[error("Ask a super admin to set the global Telegram bot first, or use your own bot.")]
    NoBot,
    #[error("Keep the name to 60 characters.")]
    NoName,
    #[error("Pick critical, warning or both.")]
    NoLevel,
    #[error("Pick new tickets, ticket moves or both.")]
    NoEvent,
    #[error("PagerDuty takes incidents only, so pick another kind for tickets.")]
    TicketPagerDuty,
    #[error("The owner's email cards stay as they are, and you can only turn them on or off.")]
    Fixed,
    #[error(
        "This section has {} cards, which is the limit, so delete one first.",
        crate::platform::caps::ALERT_CARDS
    )]
    Full,
    #[error("No such alert card.")]
    NotFound,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Queues one webhook call for an alert in `format`: to the URL, or, for the global Telegram bot,
/// to the chat, which delivery sends with the token.
async fn queue_webhook(
    c: &Connection,
    format: &str,
    url: &str,
    key: Option<&str>,
    slug: &str,
    msg: &compose::Composed,
    at: i64,
) -> anyhow::Result<()> {
    let (target, body) = format::webhook(format, url, key, slug, msg);
    let channel = if format == "telegram_bot" {
        "telegram"
    } else {
        "webhook"
    };
    store::queue(c, channel, &target, None, &body.to_string(), at).await
}

/// US-16. Queues the alert to every card that is on and listens to the event's severity.
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
    let what = if severity == "critical" {
        Listen::Critical
    } else {
        Listen::Warning
    };
    cards::queue_alert(c, project_id, &project.1, what, &msg).await?;
    Ok(())
}

/// A monitor whose HTTPS certificate expires soon (v1.1 Q25).
pub struct CertWarning<'a> {
    pub project_id: i64,
    pub monitor_id: i64,
    pub monitor_name: &'a str,
    pub expires_at: i64,
}

/// Queues a certificate that expires soon to the cards that listen to warning. Opens no
/// incident. False when nothing was queued: no project, or no card listens.
pub async fn send_cert_warning(
    c: &Connection,
    w: &CertWarning<'_>,
    base_url: &str,
) -> anyhow::Result<bool> {
    let Some(project) = crate::projects::name_slug(c, w.project_id).await? else {
        return Ok(false);
    };
    let msg = cert_warning(w, &project, base_url, now());
    cards::queue_alert(c, w.project_id, &project.1, Listen::Warning, &msg).await
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
