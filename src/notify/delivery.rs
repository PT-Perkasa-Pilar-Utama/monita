//! The outbox loop: claims due rows, sends them a few at a time, and records each outcome.

use std::{
    sync::{Arc, atomic::AtomicBool},
    time::Duration,
};

use super::{SentEmail, store};
use crate::{app::AppState, platform::now};

/// Retry delays after the 1st, 2nd and 3rd failure. The 4th failure marks the row failed.
const RETRY_DELAYS_S: [i64; 3] = [60, 300, 900];
const BATCH: i64 = 20;
const WEBHOOK_TIMEOUT: Duration = Duration::from_secs(10);
/// The cap on one send of either channel.
pub(super) const SEND_TIMEOUT: Duration = Duration::from_secs(20);
/// Sends in flight at once. A full batch takes at most 20 rows × 20 s ÷ 5 = 80 s, inside the lease.
const PARALLEL: usize = 5;
/// How long a claimed row stays invisible to other passes. Longer than a full batch can take.
const LEASE_S: i64 = 120;

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

/// Is `to` Monita's own sender address, which no user signs in with? Mail to it loops back to
/// the mailbox that sent it, so it is dropped. A super admin who signs in with that address still
/// gets their mail.
async fn to_own_sender(state: &AppState, to: &str) -> anyhow::Result<bool> {
    let c = state.db.read().await?;
    let from = match crate::platform::settings::mail(&c).await? {
        Some(m) => m.from,
        None => state.config.mail_from.clone(),
    };
    let Ok(from) = from.parse::<lettre::message::Mailbox>() else {
        return Ok(false);
    };
    if !from.email.to_string().eq_ignore_ascii_case(to.trim()) {
        return Ok(false);
    }
    Ok(!crate::accounts::email_is_active_user(&c, to).await?)
}

/// Sends one row and records the outcome at once. Returns whether it was sent.
async fn deliver_one(state: &AppState, row: store::OutboxRow) -> anyhow::Result<bool> {
    if row.channel == "email" && to_own_sender(state, &row.recipient).await? {
        tracing::info!(
            outbox_id = row.id,
            "email to Monita's own sender address dropped"
        );
        state
            .db
            .write(async |c| store::delete(c, row.id).await)
            .await?;
        return Ok(false);
    }
    let send = async {
        match row.channel.as_str() {
            "email" => {
                state
                    .mailer
                    .send(
                        &state.db,
                        SentEmail {
                            to: row.recipient.clone(),
                            subject: row.subject.clone().unwrap_or_default(),
                            body: row.body.clone(),
                        },
                    )
                    .await
            }
            "telegram" => {
                let Some(url) =
                    super::telegram::send_url(&state.db, &state.config.telegram_api).await?
                else {
                    anyhow::bail!("The global Telegram bot is not set");
                };
                super::telegram::post(state.http.webhook(), &url, &row.body).await
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
