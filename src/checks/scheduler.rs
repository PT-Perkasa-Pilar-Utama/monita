//! The one loop that drives every timed job. Spec: docs/technical-specs/13-check-scheduler.md.

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use tokio::task::JoinHandle;

use crate::{app::AppState, platform::now};

pub fn spawn(state: AppState) -> JoinHandle<()> {
    tokio::spawn(async move {
        let delivering = Arc::new(AtomicBool::new(false));
        let mut tick = tokio::time::interval(Duration::from_secs(1));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tick.tick().await;
            state.last_tick.store(now(), Ordering::Relaxed);
            crate::notify::spawn_delivery(&state, &delivering);
        }
    })
}
