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
/// How far a row waits when its webhook server already has a send in this pass.
const DEFER_S: i64 = 5;

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
    if !res.status().is_success() {
        return Err(Answered(res.status().as_u16()).into());
    }
    Ok(())
}

/// A webhook server that answered with a status other than 2xx. It answered, so it is not slow:
/// one project's stale Discord hook must not hold back every other project's Discord alerts.
#[derive(Debug, thiserror::Error)]
#[error("webhook answered HTTP {0}")]
struct Answered(u16);

/// Whether the server answered at all. Only a server that did not answer holds a send slot.
fn answered(result: &anyhow::Result<()>) -> bool {
    result
        .as_ref()
        .map_or_else(|e| e.downcast_ref::<Answered>().is_some(), |()| true)
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
    // A server that failed lately gets one row a pass, so dead servers cannot fill every slot
    // in front of other projects' alerts.
    let (due, later) = hold_back_slow(due, |s| state.slow_servers.is_slow(s, t));
    if !later.is_empty() {
        state
            .db
            .write(async |c| store::defer(c, &later, t + DEFER_S).await)
            .await?;
    }
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

/// Webhook servers whose last send got no answer, with when that stops counting. A server that never
/// answers holds a send slot for the whole timeout, so while any such server is known, one pass
/// sends at most one row to all of them together and the rest wait a few seconds. Healthy
/// servers, a shared one such as Slack's included, keep the full parallel sends. In memory: a
/// restart forgets them, and the first send after it finds them again.
#[derive(Default)]
pub struct SlowServers(std::sync::Mutex<std::collections::HashMap<String, i64>>);

/// How long a failed webhook server counts as slow.
const SLOW_FOR_S: i64 = 600;

impl SlowServers {
    fn is_slow(&self, server: &str, now: i64) -> bool {
        let map = self.0.lock().expect("slow servers lock");
        map.get(server).is_some_and(|until| *until > now)
    }

    fn record(&self, server: String, ok: bool, now: i64) {
        let mut map = self.0.lock().expect("slow servers lock");
        map.retain(|_, until| *until > now);
        if ok {
            map.remove(&server);
        } else {
            map.insert(server, now + SLOW_FOR_S);
        }
    }
}

/// The webhook server a row goes to, as `host:port`. Email and the global Telegram bot each go to
/// one shared server every project uses, so they are never held back.
fn server_of(row: &store::OutboxRow) -> Option<String> {
    if matches!(row.channel.as_str(), "email" | "telegram") {
        return None;
    }
    let u = reqwest::Url::parse(&row.recipient).ok()?;
    Some(format!(
        "{}:{}",
        u.host_str().unwrap_or(""),
        u.port_or_known_default().unwrap_or(0)
    ))
}

/// Splits claimed rows into those to send now and the ids of those that wait: of the rows to
/// slow servers, only the first goes in this pass.
fn hold_back_slow(
    rows: Vec<store::OutboxRow>,
    is_slow: impl Fn(&str) -> bool,
) -> (Vec<store::OutboxRow>, Vec<i64>) {
    let mut slow_sent = false;
    let mut now = Vec::new();
    let mut later = Vec::new();
    for row in rows {
        let slow = server_of(&row).is_some_and(|s| is_slow(&s));
        if slow && slow_sent {
            later.push(row.id);
        } else {
            slow_sent |= slow;
            now.push(row);
        }
    }
    (now, later)
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
    if let Some(server) = server_of(&row) {
        state.slow_servers.record(server, answered(&result), now());
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: i64, channel: &str, recipient: &str) -> store::OutboxRow {
        store::OutboxRow {
            id,
            channel: channel.into(),
            recipient: recipient.into(),
            subject: None,
            body: String::new(),
            attempts: 0,
        }
    }

    #[test]
    fn of_the_rows_to_slow_servers_one_goes_now_and_the_rest_wait() {
        let rows = vec![
            row(1, "webhook", "http://dead.test/a"),
            row(2, "webhook", "http://dead.test/b"),
            row(3, "webhook", "http://other-dead.test/a"),
            row(4, "webhook", "https://hooks.test/x"),
            row(5, "webhook", "https://hooks.test/y"),
            row(6, "email", "a@x.co"),
            row(7, "telegram", "-100"),
        ];
        let slow = |s: &str| s.contains("dead.test");
        let (now, later) = hold_back_slow(rows, slow);
        let ids: Vec<i64> = now.iter().map(|r| r.id).collect();
        assert_eq!(ids, [1, 4, 5, 6, 7], "healthy servers keep every row");
        assert_eq!(later, [2, 3], "all slow servers share one row a pass");
    }

    #[test]
    fn a_server_that_answers_an_error_is_not_slow() {
        assert!(answered(&Ok(())));
        assert!(answered(&Err(Answered(404).into())));
        assert!(!answered(&Err(anyhow::anyhow!(
            "send timed out after 20 s"
        ))));
    }

    #[test]
    fn a_failed_server_is_slow_until_it_answers_or_ten_minutes_pass() {
        let s = SlowServers::default();
        assert!(!s.is_slow("a:80", 100));
        s.record("a:80".into(), false, 100);
        assert!(s.is_slow("a:80", 101));
        assert!(!s.is_slow("a:80", 100 + SLOW_FOR_S));
        s.record("a:80".into(), true, 102);
        assert!(!s.is_slow("a:80", 103));
    }
}
