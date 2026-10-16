//! Once-a-day jobs, run from the scheduler tick (spec 13 section 13.2). Each job records the UTC
//! day it last finished in `settings`, so a restart neither skips nor repeats a day. A failed job
//! records its attempt and waits `RETRY_S` before the next one; the other jobs still run.

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use super::{now, settings, time};
use crate::app::AppState;

const RETRY_S: i64 = 600;

#[derive(Clone, Copy)]
enum Kind {
    Rollup,
    Backup,
    Digest,
}

struct Job {
    name: &'static str,
    /// Seconds after UTC midnight when the job becomes due.
    at_s: i64,
    kind: Kind,
}

/// Due once `at_s` has passed today, the job has not finished today, and its last failure is
/// at least `RETRY_S` old.
fn is_due(at_s: i64, t: i64, last_day: Option<&str>, last_attempt: Option<i64>) -> bool {
    t.rem_euclid(86_400) >= at_s
        && last_day != Some(time::day(t).as_str())
        && last_attempt.is_none_or(|a| t - a >= RETRY_S)
}

async fn due(state: &AppState, job: &Job, t: i64) -> anyhow::Result<bool> {
    let conn = state.db.read().await?;
    let last_day = settings::get_raw(&conn, &format!("last_{}_day", job.name)).await?;
    let last_attempt = settings::get_raw(&conn, &format!("last_{}_attempt_at", job.name))
        .await?
        .and_then(|v| v.parse().ok());
    Ok(is_due(job.at_s, t, last_day.as_deref(), last_attempt))
}

async fn record(state: &AppState, key: String, value: String) -> anyhow::Result<()> {
    state
        .db
        .write(async |c| settings::set_raw(c, &key, &value).await)
        .await
}

async fn run_one(state: &AppState, job: &Job, t: i64) -> anyhow::Result<()> {
    if !due(state, job, t).await? {
        return Ok(());
    }
    match job.kind {
        Kind::Rollup => crate::checks::retention::run(&state.db, t).await?,
        Kind::Backup => {
            super::backup::run(state, t).await?;
        }
        Kind::Digest => {
            crate::notify::digest(&state.db, &state.config.base_url, t).await?;
        }
    }
    record(state, format!("last_{}_day", job.name), time::day(t)).await
}

/// Clears the running flag when dropped, so a pass that panics does not block every later one.
pub struct Reset(Arc<AtomicBool>);

impl Drop for Reset {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

/// Sets the flag and returns its guard, or `None` when a pass is already running.
pub fn try_start(running: &Arc<AtomicBool>) -> Option<Reset> {
    (!running.swap(true, Ordering::AcqRel)).then(|| Reset(running.clone()))
}

pub fn spawn_daily(state: &AppState, running: &Arc<AtomicBool>) {
    let Some(reset) = try_start(running) else {
        return;
    };
    let state = state.clone();
    tokio::spawn(async move {
        let _reset = reset;
        run_due(&state, now()).await;
    });
}

/// Runs every job that is due at `t`. A failed job is logged and does not stop the others.
pub async fn run_due(state: &AppState, t: i64) {
    let _pass = state.gate.read().await;
    let jobs = [
        Job {
            name: "rollup",
            at_s: 10 * 60,
            kind: Kind::Rollup,
        },
        Job {
            name: "backup",
            at_s: 3_600,
            kind: Kind::Backup,
        },
        Job {
            name: "digest",
            at_s: i64::from(state.config.digest_utc_hour) * 3_600,
            kind: Kind::Digest,
        },
    ];
    for job in &jobs {
        if let Err(e) = run_one(state, job, t).await {
            tracing::error!(job = job.name, error = %e, "daily job failed");
            let key = format!("last_{}_attempt_at", job.name);
            if let Err(e) = record(state, key, t.to_string()).await {
                tracing::error!(job = job.name, error = %e, "could not record the failed attempt");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_failed_job_waits_ten_minutes_and_a_finished_one_waits_a_day() {
        let day = 1_792_108_800; // 2026-10-16 00:00 UTC
        let t = day + 3_600;
        assert!(!is_due(3_600, t - 1, None, None), "not before its hour");
        assert!(is_due(3_600, t, None, None));
        assert!(!is_due(3_600, t, Some("2026-10-16"), None), "done today");
        assert!(is_due(3_600, t, Some("2026-10-15"), None));
        assert!(!is_due(3_600, t + 599, None, Some(t)), "retry too soon");
        assert!(is_due(3_600, t + 600, None, Some(t)));
    }

    #[tokio::test]
    async fn the_running_flag_clears_when_a_pass_panics() {
        let running = Arc::new(AtomicBool::new(false));
        let reset = try_start(&running).unwrap();
        assert!(try_start(&running).is_none(), "one pass at a time");
        let pass = tokio::spawn(async move {
            let _reset = reset;
            panic!("job bug");
        });
        assert!(pass.await.is_err());
        assert!(!running.load(Ordering::Acquire));
    }
}
