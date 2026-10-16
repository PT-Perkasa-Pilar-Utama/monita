//! notify module: alert routes, the outbox, delivery.
//! Spec: docs/technical-specs/05-module-definitions.md section 5.5.
//! Callers queue messages inside their own write transaction; the outbox loop delivers them,
//! so a slow SMTP server never blocks a check or a request.

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

use turso::Connection;

use crate::{app::AppState, platform::now};

/// Retry delays after the 1st, 2nd and 3rd failure. The 4th failure marks the row failed.
const RETRY_DELAYS_S: [i64; 3] = [60, 300, 900];
const BATCH: i64 = 20;

/// Queues one email. Call it inside `Db::write` with the change that caused it.
pub async fn queue_email(
    c: &Connection,
    to: &str,
    subject: &str,
    body: &str,
    at: i64,
) -> anyhow::Result<()> {
    c.execute(
        "INSERT INTO outbox (channel, recipient, subject, body, next_attempt_at) VALUES ('email', ?1, ?2, ?3, ?4)",
        (to, subject, body, at),
    )
    .await?;
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SentEmail {
    pub to: String,
    pub subject: String,
    pub body: String,
}

/// The mail port. `Log` stands in when no SMTP server is set; `Memory` records for tests.
// ponytail: no SMTP adapter until Sprint 2 (US-16); `MONITA_SMTP_URL` is read but unused.
#[derive(Clone)]
pub enum Mailer {
    Log,
    Memory(Arc<Mutex<Vec<SentEmail>>>),
}

impl Mailer {
    pub fn memory() -> Self {
        Mailer::Memory(Arc::default())
    }

    pub fn sent(&self) -> Vec<SentEmail> {
        match self {
            Mailer::Memory(m) => m.lock().expect("mailer lock").clone(),
            Mailer::Log => Vec::new(),
        }
    }

    async fn send(&self, email: SentEmail) -> anyhow::Result<()> {
        match self {
            Mailer::Log => {
                tracing::info!(to = %email.to, subject = %email.subject, body = %email.body, "email (no SMTP configured)");
                Ok(())
            }
            Mailer::Memory(m) => {
                m.lock().expect("mailer lock").push(email);
                Ok(())
            }
        }
    }
}

/// Starts one delivery pass unless one is already running.
pub fn spawn_delivery(state: &AppState, running: &Arc<AtomicBool>) {
    if running.swap(true, Ordering::AcqRel) {
        return;
    }
    let state = state.clone();
    let running = running.clone();
    tokio::spawn(async move {
        if let Err(e) = deliver_due(&state).await {
            tracing::error!(error = %e, "outbox delivery failed");
        }
        running.store(false, Ordering::Release);
    });
}

/// Delivers due email rows. Returns how many were sent.
pub async fn deliver_due(state: &AppState) -> anyhow::Result<usize> {
    let t = now();
    let due = {
        let conn = state.db.read().await?;
        let mut rows = conn
            .query(
                "SELECT id, recipient, subject, body, attempts FROM outbox
                 WHERE channel = 'email' AND sent_at IS NULL AND failed_at IS NULL AND next_attempt_at <= ?1
                 ORDER BY next_attempt_at LIMIT ?2",
                (t, BATCH),
            )
            .await?;
        let mut due = Vec::new();
        while let Some(r) = rows.next().await? {
            due.push((
                r.get::<i64>(0)?,
                SentEmail {
                    to: r.get(1)?,
                    subject: r.get::<Option<String>>(2)?.unwrap_or_default(),
                    body: r.get(3)?,
                },
                r.get::<i64>(4)?,
            ));
        }
        due
    };

    let mut sent = 0;
    for (id, email, attempts) in due {
        let result = state.mailer.send(email).await;
        state
            .db
            .write(async |c| {
                match &result {
                    Ok(()) => {
                        c.execute("UPDATE outbox SET sent_at = ?1, attempts = attempts + 1 WHERE id = ?2", (now(), id))
                            .await?;
                    }
                    Err(e) => match RETRY_DELAYS_S.get(attempts as usize) {
                        Some(delay) => {
                            c.execute(
                                "UPDATE outbox SET attempts = attempts + 1, next_attempt_at = ?1, last_error = ?2 WHERE id = ?3",
                                (now() + delay, e.to_string(), id),
                            )
                            .await?;
                        }
                        None => {
                            c.execute(
                                "UPDATE outbox SET attempts = attempts + 1, failed_at = ?1, last_error = ?2 WHERE id = ?3",
                                (now(), e.to_string(), id),
                            )
                            .await?;
                            tracing::error!(outbox_id = id, error = %e, "email failed after retries");
                        }
                    },
                }
                Ok(())
            })
            .await?;
        if result.is_ok() {
            sent += 1;
        }
    }
    Ok(sent)
}
