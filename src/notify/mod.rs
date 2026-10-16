//! notify module: alert routes, the outbox, delivery.
//! Spec: docs/technical-specs/05-module-definitions.md section 5.5, api-specs 06.
//! Callers queue messages inside their own write transaction; the outbox loop delivers them,
//! so a slow SMTP server or webhook never blocks a check or a request.

mod digest;
mod mailer;
pub mod routes;
mod store;

use std::{
    sync::{Arc, atomic::AtomicBool},
    time::Duration,
};

use turso::Connection;

use crate::{
    app::AppState,
    incidents::Event,
    platform::{db::Db, now, time},
};

pub use digest::digest;
pub use mailer::{Mailer, SentEmail};
pub use store::Route;

/// Retry delays after the 1st, 2nd and 3rd failure. The 4th failure marks the row failed.
const RETRY_DELAYS_S: [i64; 3] = [60, 300, 900];
const BATCH: i64 = 20;
const WEBHOOK_TIMEOUT: Duration = Duration::from_secs(10);
/// The cap on one send of either channel.
const SEND_TIMEOUT: Duration = Duration::from_secs(20);
/// Sends in flight at once. A full batch takes at most 20 rows × 20 s ÷ 5 = 80 s, inside the lease.
const PARALLEL: usize = 5;
/// How long a claimed row stays invisible to other passes. Longer than a full batch can take.
const LEASE_S: i64 = 120;

/// AC-15.02: a new project's two routes. Critical calls its webhook once one is set; warning does not.
pub async fn create_routes(c: &Connection, project_id: i64) -> anyhow::Result<()> {
    store::create_routes(c, project_id).await
}

/// Retention (US-32): finished rows older than `cutoff`. Runs inside the caller's write.
pub async fn purge_delivered(c: &Connection, cutoff: i64) -> anyhow::Result<()> {
    store::purge_delivered(c, cutoff).await
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
    let webhook = webhook_raw.trim();
    if !webhook.is_empty() && !(webhook.starts_with("http://") || webhook.starts_with("https://")) {
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

struct Composed {
    subject: String,
    text: String,
    json: serde_json::Value,
}

fn compose(ev: &Event, project: &(String, String), base_url: &str) -> (String, Composed) {
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
    let body = serde_json::json!({
        "text": format!("[TICKET] {name} #{number}: {title}"),
        "event": "ticket.created",
        "project": slug,
        "ticket_number": number,
        "title": title,
        "ticket_url": format!("{base_url}/projects/{slug}/tickets/{number}"),
        "at": now(),
    });
    store::queue(c, "webhook", url, None, &body.to_string(), now()).await
}

/// The test button on the alerts page: one message per recipient of that severity.
pub async fn send_test(
    db: &Db,
    project_id: i64,
    severity: &str,
    base_url: &str,
) -> anyhow::Result<usize> {
    db.write(async |c| {
        let Some((name, slug)) = crate::projects::name_slug(c, project_id).await? else { return Ok(0) };
        let Some(route) = store::route(c, project_id, severity).await? else { return Ok(0) };
        let text = format!("[TEST] {name}: alerts for {severity} reach this channel.");
        let at = now();
        let mut n = 0;
        for email in &route.emails {
            store::queue(c, "email", email, Some(&text), &format!("{text}\n\n{base_url}/projects/{slug}/alerts\n"), at).await?;
            n += 1;
        }
        if let Some(url) = route.webhook_url.as_deref().filter(|_| route.use_webhook) {
            let body = serde_json::json!({ "text": text, "event": "test", "project": slug, "severity": severity, "at": at });
            store::queue(c, "webhook", url, None, &body.to_string(), at).await?;
            n += 1;
        }
        Ok(n)
    })
    .await
}

/// Read-only: is any outbox row due at `at`? The scheduler asks before it spawns a delivery pass.
pub async fn any_due(c: &Connection, at: i64) -> anyhow::Result<bool> {
    store::any_due(c, at).await
}

pub fn spawn_delivery(state: &AppState, running: &Arc<AtomicBool>) {
    let Some(reset) = crate::platform::jobs::try_start(running) else {
        return;
    };
    let state = state.clone();
    tokio::spawn(async move {
        let _reset = reset;
        let _pass = state.gate.read().await;
        if let Err(e) = deliver_due(&state).await {
            tracing::error!(error = %e, "outbox delivery failed");
        }
    });
}

async fn post_webhook(state: &AppState, url: &str, body: &str) -> anyhow::Result<()> {
    let res = state
        .http
        .webhook()
        .post(url)
        .header("content-type", "application/json")
        .body(body.to_string())
        .timeout(WEBHOOK_TIMEOUT)
        .send()
        .await
        // The URL is the credential for Slack-style hooks (SEC-20).
        .map_err(|e| anyhow::anyhow!("{}", e.without_url()))?;
    // A redirect is a failure: following it would turn the POST into a GET without the payload.
    anyhow::ensure!(
        res.status().is_success(),
        "webhook answered HTTP {}",
        res.status().as_u16()
    );
    Ok(())
}

/// Delivers due rows of both channels, `PARALLEL` at a time. Returns how many were sent.
pub async fn deliver_due(state: &AppState) -> anyhow::Result<usize> {
    let t = now();
    if !store::any_due(&state.db.read().await?, t).await? {
        return Ok(0);
    }
    let due = state
        .db
        .write(async |c| store::claim_due(c, t, BATCH, t + LEASE_S).await)
        .await?;
    let mut queue = due.into_iter();
    let mut running = tokio::task::JoinSet::new();
    let mut sent = 0;
    loop {
        while running.len() < PARALLEL
            && let Some(row) = queue.next()
        {
            let state = state.clone();
            running.spawn(async move { deliver_one(&state, row).await });
        }
        let Some(done) = running.join_next().await else {
            break;
        };
        // One row's failure must not cancel the others: its row stays leased and retries.
        match done {
            Ok(Ok(true)) => sent += 1,
            Ok(Ok(false)) => {}
            Ok(Err(e)) => {
                tracing::error!(error = %e, "could not record a delivery; the row retries after its lease");
            }
            Err(e) => tracing::error!(error = %e, "a delivery task panicked"),
        }
    }
    Ok(sent)
}

/// Sends one row and records the outcome at once. Returns whether it was sent.
async fn deliver_one(state: &AppState, row: store::OutboxRow) -> anyhow::Result<bool> {
    let send = async {
        match row.channel.as_str() {
            "email" => {
                state
                    .mailer
                    .send(SentEmail {
                        to: row.recipient.clone(),
                        subject: row.subject.clone().unwrap_or_default(),
                        body: row.body.clone(),
                    })
                    .await
            }
            _ => post_webhook(state, &row.recipient, &row.body).await,
        }
    };
    let result = tokio::time::timeout(SEND_TIMEOUT, send)
        .await
        .unwrap_or_else(|_| Err(anyhow::anyhow!("send timed out after 20 s")));
    let outcome = match &result {
        Ok(()) => store::Outcome::Sent,
        Err(e) => match RETRY_DELAYS_S.get(row.attempts as usize) {
            Some(delay) => store::Outcome::Retry(now() + delay, e.to_string()),
            None => {
                tracing::error!(outbox_id = row.id, channel = %row.channel, error = %e, "delivery failed after retries");
                store::Outcome::Failed(e.to_string())
            }
        },
    };
    state
        .db
        .write(async |c| store::mark(c, row.id, &outcome, now()).await)
        .await?;
    Ok(result.is_ok())
}
