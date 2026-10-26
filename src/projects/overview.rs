//! A project's Overview: where its monitors, heartbeats, tickets and incidents stand.

use serde::Serialize;

use crate::{
    checks::{self, Summary, chart::overview::OverviewChart},
    incidents::{self, IncidentRow},
    platform::{db::Db, now},
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
    /// The last 24 hours of every monitor and heartbeat on one chart (v1.1 Q39).
    pub chart: OverviewChart,
}

/// `off`: the viewer's clock, minutes east of UTC, for the chart's labels. `viewer`: whose pins
/// lead the chart, none for an API key.
pub async fn read(
    db: &Db,
    project_id: i64,
    viewer: Option<i64>,
    off: i64,
) -> anyhow::Result<Overview> {
    let [monitors, heartbeats] = checks::summaries(db, project_id).await?;
    let stages = tickets::stage_counts(db, project_id).await?;
    let triage = stages
        .iter()
        .find(|(k, _)| k == "triage")
        .map_or(0, |(_, n)| *n);
    let mut open = incidents::list(db, project_id, Some(true)).await?;
    open.truncate(INCIDENTS);
    let keys = super::list_keys(db, project_id).await?;
    let chart = checks::series::project(db, project_id, viewer, now(), off).await?;
    Ok(Overview {
        chart,
        monitors,
        heartbeats,
        triage,
        incidents: open,
        active_keys: keys.iter().filter(|k| k.revoked_at.is_none()).count(),
    })
}
