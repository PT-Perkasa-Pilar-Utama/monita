//! A project's Overview: where its monitors, heartbeats, tickets and incidents stand.

use std::collections::HashMap;

use serde::Serialize;

use super::pulse::{self, Hour};
use crate::{
    checks::{self, Brief, Summary},
    incidents::{self, IncidentRow, SourceKind},
    platform::{db::Db, now},
    tickets,
};

/// Open incidents shown on the page; the Incidents tab lists the rest.
const INCIDENTS: usize = 5;
/// Rows of each kind on the timeline: the ones that need the team first.
const LANES: usize = 10;

/// One monitor or heartbeat on the Overview's timeline: a bar an hour for the last day.
#[derive(Serialize)]
pub struct Lane {
    pub id: i64,
    pub name: String,
    pub state: &'static str,
    /// The share of the last 24 hours it was watched and not down; null when not watched.
    pub uptime: Option<f64>,
    pub hours: Vec<Hour>,
}

/// The last 24 hours of the project's monitors and heartbeats, `LANES` of each at most.
#[derive(Serialize)]
pub struct Timeline {
    pub monitors: Vec<Lane>,
    pub heartbeats: Vec<Lane>,
}

/// Down first, then an issue, maintenance, waiting, up, and paused last.
fn rank(state: &str) -> u8 {
    match state {
        "down" => 0,
        "issue" => 1,
        "maintenance" => 2,
        "unknown" => 3,
        "up" => 4,
        _ => 5,
    }
}

/// The lanes that need the team first: by state, then the lowest uptime, then by name.
fn sort_lanes(v: &mut [Lane]) {
    v.sort_by(|a, b| {
        rank(a.state)
            .cmp(&rank(b.state))
            .then(
                a.uptime
                    .unwrap_or(f64::INFINITY)
                    .total_cmp(&b.uptime.unwrap_or(f64::INFINITY)),
            )
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
}

type Spans = HashMap<(SourceKind, i64), Vec<(i64, i64)>>;

fn lanes(sources: &[Brief], kind: SourceKind, spans: &Spans, t: i64, off: i64) -> Vec<Lane> {
    let mut v: Vec<Lane> = sources
        .iter()
        .map(|b| {
            let down = spans.get(&(kind, b.id)).map_or(&[][..], Vec::as_slice);
            Lane {
                id: b.id,
                name: b.name.clone(),
                state: b.state,
                uptime: pulse::share_up(down, b.watched, t - 86_400, t),
                hours: pulse::hours(down, &[b.watched], t, off),
            }
        })
        .collect();
    sort_lanes(&mut v);
    v.truncate(LANES);
    v
}

#[derive(Serialize)]
pub struct Overview {
    pub monitors: Summary,
    pub heartbeats: Summary,
    pub triage: i64,
    /// The newest open incidents, at most `INCIDENTS`.
    pub incidents: Vec<IncidentRow>,
    pub active_keys: usize,
    pub timeline: Timeline,
}

pub async fn read(db: &Db, project_id: i64) -> anyhow::Result<Overview> {
    let [monitors, heartbeats] = checks::summaries(db, project_id).await?;
    let stages = tickets::stage_counts(db, project_id).await?;
    let triage = stages
        .iter()
        .find(|(k, _)| k == "triage")
        .map_or(0, |(_, n)| *n);
    let mut open = incidents::list(db, project_id, Some(true)).await?;
    open.truncate(INCIDENTS);
    let keys = super::list_keys(db, project_id).await?;
    let t = now();
    let off = crate::platform::settings::utc_offset(&db.read().await?).await?;
    let spans = incidents::history::intervals(db, project_id, t - 86_400, t).await?;
    let timeline = Timeline {
        monitors: lanes(&monitors.sources, SourceKind::Monitor, &spans, t, off),
        heartbeats: lanes(&heartbeats.sources, SourceKind::Heartbeat, &spans, t, off),
    };
    Ok(Overview {
        timeline,
        monitors,
        heartbeats,
        triage,
        incidents: open,
        active_keys: keys.iter().filter(|k| k.revoked_at.is_none()).count(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lane(name: &str, state: &'static str, uptime: Option<f64>) -> Lane {
        Lane {
            id: 0,
            name: name.into(),
            state,
            uptime,
            hours: Vec::new(),
        }
    }

    #[test]
    fn the_lanes_that_need_the_team_come_first() {
        let mut v = vec![
            lane("paused", "paused", None),
            lane("b steady", "up", Some(100.0)),
            lane("A steady", "up", Some(100.0)),
            lane("flaky", "up", Some(97.5)),
            lane("new", "unknown", None),
            lane("down", "down", Some(99.0)),
            lane("issue", "issue", Some(100.0)),
        ];
        sort_lanes(&mut v);
        let names: Vec<&str> = v.iter().map(|l| l.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "down", "issue", "new", "flaky", "A steady", "b steady", "paused"
            ]
        );
    }
}
