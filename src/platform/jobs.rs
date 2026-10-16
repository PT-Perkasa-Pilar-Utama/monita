//! Once-a-day jobs, run from the scheduler tick (spec 13 section 13.2). Each job records the UTC
//! day it last finished in `settings`, so a restart neither skips nor repeats a day.

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use super::{now, settings, time};
use crate::app::AppState;

struct Job {
    name: &'static str,
    /// Seconds after UTC midnight when the job becomes due.
    at_s: i64,
}

const ROLLUP: Job = Job {
    name: "rollup",
    at_s: 10 * 60,
};
const BACKUP: Job = Job {
    name: "backup",
    at_s: 3_600,
};

async fn due(state: &AppState, job: &Job, t: i64) -> anyhow::Result<bool> {
    if t.rem_euclid(86_400) < job.at_s {
        return Ok(false);
    }
    let last =
        settings::get_raw(&state.db.read().await?, &format!("last_{}_day", job.name)).await?;
    Ok(last.as_deref() != Some(time::day(t).as_str()))
}

async fn done(state: &AppState, job: &Job, t: i64) -> anyhow::Result<()> {
    let (key, day) = (format!("last_{}_day", job.name), time::day(t));
    state
        .db
        .write(async |c| settings::set_raw(c, &key, &day).await)
        .await
}

/// Starts one pass over the daily jobs unless one is already running.
pub fn spawn_daily(state: &AppState, running: &Arc<AtomicBool>) {
    if running.swap(true, Ordering::AcqRel) {
        return;
    }
    let (state, running) = (state.clone(), running.clone());
    tokio::spawn(async move {
        let t = now();
        if let Err(e) = run_due(&state, t).await {
            tracing::error!(error = %e, "daily jobs failed");
        }
        running.store(false, Ordering::Release);
    });
}

async fn run_due(state: &AppState, t: i64) -> anyhow::Result<()> {
    let _pass = state.gate.read().await;
    if due(state, &ROLLUP, t).await? {
        crate::checks::retention::run(&state.db, t).await?;
        done(state, &ROLLUP, t).await?;
    }
    if due(state, &BACKUP, t).await? {
        super::backup::run(state, t).await?;
        done(state, &BACKUP, t).await?;
    }
    Ok(())
}
