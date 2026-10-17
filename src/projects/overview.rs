//! A project's Overview: where its monitors, heartbeats, tickets and incidents stand.

use serde::Serialize;

use crate::{
    checks::{self, Summary},
    incidents::{self, IncidentRow},
    platform::db::Db,
    tickets,
};

/// Open incidents shown on the page; the Incidents tab lists the rest.
const INCIDENTS: usize = 5;

#[derive(Serialize)]
pub struct Overview {
    pub monitors: Summary,
    pub heartbeats: Summary,
    pub triage: i64,
    /// The newest open incidents, at most `INCIDENTS`.
    pub incidents: Vec<IncidentRow>,
    pub active_keys: usize,
}

pub async fn read(db: &Db, project_id: i64) -> anyhow::Result<Overview> {
    let [monitors, heartbeats] = checks::summaries(db, project_id).await?;
    let stages = tickets::stage_counts(db, project_id).await?;
    let triage = stages
        .iter()
        .find(|(k, _)| k == "triage")
        .map_or(0, |(_, n)| *n);
    let mut open = incidents::list(db, project_id, true).await?;
    open.truncate(INCIDENTS);
    let keys = super::list_keys(db, project_id).await?;
    Ok(Overview {
        monitors,
        heartbeats,
        triage,
        incidents: open,
        active_keys: keys.iter().filter(|k| k.revoked_at.is_none()).count(),
    })
}
