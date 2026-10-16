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

/// AC-34.03: tell the peer Monita (or an outside heartbeat) that this one is alive.
const PEER_PING_EVERY_S: i64 = 60;

fn spawn_peer_ping(state: &AppState) {
    let Some(url) = state.config.ping_url.clone() else {
        return;
    };
    let client = state.http.webhook().clone();
    tokio::spawn(async move {
        let sent = client
            .get(&url)
            .timeout(Duration::from_secs(10))
            .send()
            .await;
        match sent {
            Ok(r) if r.status().is_success() => {}
            Ok(r) => tracing::warn!(
                status = r.status().as_u16(),
                "peer ping answered with an error"
            ),
            Err(e) => tracing::warn!(error = %e.without_url(), "peer ping failed"),
        }
    });
}

pub fn spawn(state: AppState) -> JoinHandle<()> {
    tokio::spawn(async move {
        let delivering = Arc::new(AtomicBool::new(false));
        let daily = Arc::new(AtomicBool::new(false));
        let mut last_peer_ping = 0i64;
        let slots = Arc::new(Semaphore::new(0));
        let mut cap = 0usize;
        let mut ticks = 0u64;
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
            // Daily jobs have minute precision at best; tick 0 checks them at start.
            if ticks.is_multiple_of(60) {
                crate::platform::jobs::spawn_daily(&state, &daily);
            }
            ticks += 1;
            if state.config.ping_url.is_some() && now() - last_peer_ping >= PEER_PING_EVERY_S {
                last_peer_ping = now();
                spawn_peer_ping(&state);
            }
        }
    })
}

/// Grows or shrinks the slot pool to `check_concurrency`. Returns the new cap.
async fn resize(state: &AppState, slots: &Semaphore, cap: usize) -> usize {
    let wanted = match state.db.read().await {
        Ok(conn) => settings::get_or(&conn, "check_concurrency", 50).await,
        Err(e) => {
            tracing::warn!(error = %e, "no read connection; check_concurrency unchanged");
            return cap;
        }
    }
    .clamp(1, 500) as usize;
    reshape(slots, cap, wanted)
}

/// `forget_permits` removes only free permits, so a shrink while probes hold slots finishes over
/// later ticks. Returns the number of permits that now exist.
fn reshape(slots: &Semaphore, cap: usize, wanted: usize) -> usize {
    if wanted > cap {
        slots.add_permits(wanted - cap);
        wanted
    } else {
        cap - slots.forget_permits(cap - wanted)
    }
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
    if !store::any_due(&state.db.read().await?, t).await? {
        return Ok(());
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shrink_while_slots_are_held_finishes_once_they_are_free() {
        let slots = Semaphore::new(5);
        let held: Vec<_> = (0..4).map(|_| slots.try_acquire().unwrap()).collect();
        let cap = reshape(&slots, 5, 2);
        assert_eq!(cap, 4, "only the one free permit could go");
        drop(held);
        let cap = reshape(&slots, cap, 2);
        assert_eq!((cap, slots.available_permits()), (2, 2));
        assert_eq!(reshape(&slots, cap, 3), 3);
        assert_eq!(slots.available_permits(), 3);
    }
}
