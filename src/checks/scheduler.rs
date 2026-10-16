//! The one loop that drives every timed job. Spec: docs/technical-specs/13-check-scheduler.md.

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use tokio::{
    sync::{OwnedRwLockReadGuard, Semaphore},
    task::JoinHandle,
};

use super::store;
use crate::{
    app::AppState,
    platform::{now, settings},
};

pub fn spawn(state: AppState) -> JoinHandle<()> {
    tokio::spawn(async move {
        let delivering = Arc::new(AtomicBool::new(false));
        let slots = Arc::new(Semaphore::new(0));
        let mut cap = 0usize;
        let mut tick = tokio::time::interval(Duration::from_secs(1));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tick.tick().await;
            state.last_tick.store(now(), Ordering::Relaxed);
            // One read guard per tick, shared with the probes it starts (a second read could
            // queue behind a waiting reset and deadlock).
            let pass = Arc::new(state.gate.clone().read_owned().await);
            // A primary-key read; a changed cap applies on the next tick.
            cap = resize(&state, &slots, cap).await;
            if let Err(e) = run_due_monitors(&state, &slots, &pass).await {
                tracing::error!(error = %e, "scheduler: monitor pass failed");
            }
            if let Err(e) =
                super::heartbeat::sweep_missed(&state.db, &state.config.base_url, now()).await
            {
                tracing::error!(error = %e, "scheduler: heartbeat sweep failed");
            }
            crate::notify::spawn_delivery(&state, &delivering);
        }
    })
}

/// Grows or shrinks the slot pool to `check_concurrency`. Returns the new cap.
async fn resize(state: &AppState, slots: &Semaphore, cap: usize) -> usize {
    let wanted = match state.db.read().await {
        Ok(conn) => settings::get(&conn, "check_concurrency")
            .await
            .unwrap_or(50),
        Err(_) => return cap,
    }
    .clamp(1, 500) as usize;
    if wanted > cap {
        slots.add_permits(wanted - cap);
    } else if wanted < cap {
        slots.forget_permits(cap - wanted);
    }
    wanted
}

/// Claims as many due monitors as there are free slots, and probes each in its own task.
/// Only this loop takes permits, so every claimed monitor gets one.
async fn run_due_monitors(
    state: &AppState,
    slots: &Arc<Semaphore>,
    pass: &Arc<OwnedRwLockReadGuard<()>>,
) -> anyhow::Result<()> {
    let free = slots.available_permits() as i64;
    if free == 0 {
        return Ok(());
    }
    let t = now();
    let due = state
        .db
        .write(async |c| store::claim_due(c, t, free).await)
        .await?;
    for monitor in due {
        let Ok(permit) = slots.clone().try_acquire_owned() else {
            break;
        };
        let state = state.clone();
        let held = pass.clone();
        tokio::spawn(async move {
            let _held = held;
            let result = super::probe(&monitor.target(), &state.http).await;
            drop(permit);
            let stored =
                super::record_check(&state.db, &state.config.base_url, &monitor, &result).await;
            if let Err(e) = stored {
                tracing::error!(monitor = monitor.id, error = %e, "scheduler: could not store a check");
            }
        });
    }
    Ok(())
}
