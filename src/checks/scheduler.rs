//! The one loop that drives every timed job. Spec: docs/technical-specs/13-check-scheduler.md.

use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
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
        let holds = Held::default();
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
            // One read connection serves the tick's cheap checks. `healthy` is false when any
            // database step failed, and then the peer is not told Monita is alive.
            let mut healthy = false;
            match state.db.read().await {
                Ok(conn) => {
                    // A primary-key read; a changed cap applies on the next tick.
                    cap = resize(&conn, &slots, cap).await;
                    healthy = true;
                    if let Err(e) =
                        run_due_monitors(&state, &conn, &slots, &holds, cap, &pass).await
                    {
                        tracing::error!(error = %e, "scheduler: monitor pass failed");
                        healthy = false;
                    }
                    let base_url = &state.config.base_url;
                    if let Err(e) =
                        super::heartbeat::sweep_missed(&state.db, &conn, base_url, now()).await
                    {
                        tracing::error!(error = %e, "scheduler: heartbeat sweep failed");
                        healthy = false;
                    }
                    match crate::notify::any_due(&conn, now()).await {
                        Ok(true) => crate::notify::spawn_delivery(&state, &delivering),
                        Ok(false) => {}
                        Err(e) => {
                            tracing::error!(error = %e, "scheduler: outbox check failed");
                            healthy = false;
                        }
                    }
                }
                Err(e) => tracing::error!(error = %e, "scheduler: no read connection this tick"),
            }
            // Daily jobs have minute precision at best; tick 0 checks them at start.
            if ticks.is_multiple_of(60) {
                crate::platform::jobs::spawn_daily(&state, &daily);
            }
            ticks += 1;
            if healthy
                && state.config.ping_url.is_some()
                && now() - last_peer_ping >= PEER_PING_EVERY_S
            {
                last_peer_ping = now();
                spawn_peer_ping(&state);
            }
        }
    })
}

/// Grows or shrinks the slot pool to `check_concurrency`. Returns the new cap.
async fn resize(conn: &crate::platform::sql::Connection, slots: &Semaphore, cap: usize) -> usize {
    let wanted = settings::get_or(conn, "check_concurrency", 50)
        .await
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

/// Check slots each project holds now, by project id.
type Held = Arc<Mutex<HashMap<i64, usize>>>;

/// One project's hold on a slot, given back when the probe ends, even on an error path.
struct Hold(Held, i64);

impl Hold {
    fn take(held: &Held, project: i64) -> Self {
        *held.lock().unwrap().entry(project).or_default() += 1;
        Self(held.clone(), project)
    }
}

impl Drop for Hold {
    fn drop(&mut self) {
        let mut map = self.0.lock().unwrap();
        if let Some(n) = map.get_mut(&self.1) {
            *n -= 1;
            if *n == 0 {
                map.remove(&self.1);
            }
        }
    }
}

/// Which due monitors take the `free` slots, in the order given. While another project waits,
/// no project holds more than half of the `cap` slots, so one project's slow or silent servers
/// cannot hold up every other project's checks. A slot nobody else wants still goes to a project
/// past its half. With one slot there is no half to keep.
fn pick<T>(
    found: Vec<T>,
    project: impl Fn(&T) -> i64,
    held: &HashMap<i64, usize>,
    cap: usize,
    free: usize,
) -> Vec<T> {
    let share = (cap / 2).max(1);
    let mut taken: HashMap<i64, usize> = HashMap::new();
    let (mut picked, mut waiting) = (Vec::new(), Vec::new());
    for m in found {
        if picked.len() == free {
            break;
        }
        let p = project(&m);
        let n = taken
            .entry(p)
            .or_insert_with(|| held.get(&p).copied().unwrap_or(0));
        if *n < share {
            *n += 1;
            picked.push(m);
        } else {
            waiting.push(m);
        }
    }
    let room = free - picked.len();
    picked.extend(waiting.into_iter().take(room));
    picked
}

/// Claims due monitors for the free slots, as `pick` shares them, and probes each in its own
/// task. Only this loop takes permits, so every claimed monitor gets one.
async fn run_due_monitors(
    state: &AppState,
    conn: &crate::platform::sql::Connection,
    slots: &Arc<Semaphore>,
    holds: &Held,
    cap: usize,
    pass: &Arc<OwnedRwLockReadGuard<()>>,
) -> anyhow::Result<()> {
    let free = slots.available_permits();
    if free == 0 {
        return Ok(());
    }
    let t = now();
    if !store::any_due(conn, t).await? {
        return Ok(());
    }
    // A copy: a probe that ends meanwhile only makes this pick a little cautious.
    let now_held = holds.lock().unwrap().clone();
    let due = state
        .db
        .write(async |c| {
            let found = store::due(c, t, cap as i64).await?;
            let picked = pick(found, |m| m.project_id, &now_held, cap, free);
            store::claim(c, t, &picked).await?;
            Ok(picked)
        })
        .await?;
    for monitor in due {
        let Ok(permit) = slots.clone().try_acquire_owned() else {
            break;
        };
        let hold = Hold::take(holds, monitor.project_id);
        let state = state.clone();
        let held = pass.clone();
        tokio::spawn(async move {
            let _held = held;
            let result = super::run_check(&state.db, &state.http, &monitor).await;
            drop(permit);
            drop(hold);
            let result = match result {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!(monitor = monitor.id, error = %e, "scheduler: could not read a monitor's secret");
                    return;
                }
            };
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

    #[test]
    fn a_project_holds_at_most_half_the_slots_while_another_waits() {
        let by_letter = |m: &&str| i64::from(m.as_bytes()[0]);
        let a = i64::from(b'A');
        let none = HashMap::new();
        let a_holds_one = HashMap::from([(a, 1)]);
        assert_eq!(
            pick(vec!["A1", "B1"], by_letter, &a_holds_one, 2, 1),
            ["B1"]
        );
        assert_eq!(
            pick(vec!["A1", "A2", "B1"], by_letter, &none, 2, 2),
            ["A1", "B1"]
        );
        assert_eq!(
            pick(vec!["A1", "A2"], by_letter, &none, 2, 2),
            ["A1", "A2"],
            "a slot nobody else wants still goes to A"
        );
        assert_eq!(pick(vec!["A1"], by_letter, &a_holds_one, 1, 1), ["A1"]);
        let a_holds_25 = HashMap::from([(a, 25)]);
        assert_eq!(
            pick(vec!["A9", "B1"], by_letter, &a_holds_25, 50, 3),
            ["B1", "A9"]
        );
    }
}
