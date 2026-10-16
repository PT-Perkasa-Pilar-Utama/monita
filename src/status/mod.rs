//! status module: the public status page and its JSON (US-21, US-22).
//! Spec: docs/api-specs/07-status.md. Owns no tables; reads through checks, incidents and projects.

pub mod routes;

use serde::Serialize;

use crate::{
    checks,
    incidents::{self, ItemRef, ItemState, history},
    platform::{db::Db, now, time},
    projects,
};

#[derive(Serialize)]
pub struct Status {
    pub project: ProjectRef,
    pub overall: &'static str,
    pub items: Vec<ItemStatus>,
    pub incidents: Vec<IncidentStatus>,
}

#[derive(Serialize)]
pub struct ProjectRef {
    pub name: String,
    pub slug: String,
}

#[derive(Serialize)]
pub struct ItemStatus {
    pub kind: &'static str,
    pub name: String,
    pub state: &'static str,
    /// Over the requested window.
    pub uptime_90d: Option<f64>,
    pub days: Vec<Day>,
}

#[derive(Serialize)]
pub struct Day {
    pub day: String,
    /// `null` for a day before the item existed.
    pub uptime: Option<f64>,
    pub downtime_s: i64,
}

#[derive(Serialize)]
pub struct IncidentStatus {
    pub title: String,
    pub origin: String,
    pub severity: String,
    pub started_at: i64,
    pub resolved_at: Option<i64>,
    pub items: Vec<String>,
    pub updates: Vec<Update>,
}

#[derive(Serialize)]
pub struct Update {
    pub label: String,
    pub message: String,
    pub at: i64,
}

fn round2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

/// Uptime % over `[from, to)` with `down` seconds of downtime; `None` when the span is empty.
fn uptime(down: i64, from: i64, to: i64) -> Option<f64> {
    let span = to - from;
    (span > 0).then(|| round2(100.0 * (1.0 - down.min(span) as f64 / span as f64)))
}

/// Everything the page and the JSON show for `slug` over the last `days` UTC days, or `None`.
pub async fn load(db: &Db, slug: &str, days: usize) -> anyhow::Result<Option<Status>> {
    let Some(project) = projects::find_public(db, slug).await? else {
        return Ok(None);
    };
    let t = now();
    let first_day = crate::platform::time::day_start(t) - (days as i64 - 1) * 86_400;
    let public = checks::public_items(db, project.id).await?;
    let states = incidents::states_for(db, project.id).await?;
    let intervals = history::intervals(db, project.id, first_day, t).await?;

    let items: Vec<ItemStatus> = public
        .iter()
        .map(|item| {
            let key = (item.kind, item.id);
            let spans = intervals.get(&key).map(Vec::as_slice).unwrap_or(&[]);
            let per_day = history::per_day(spans, first_day, days);
            let mut total_down = 0;
            let day_list = per_day
                .iter()
                .enumerate()
                .map(|(i, &down)| {
                    let start = first_day + i as i64 * 86_400;
                    // Null only for a day that ended before the item existed; an item created
                    // a moment ago has had no downtime yet, so its first day reads 100%.
                    let up = (start + 86_400 > item.created_at).then(|| {
                        uptime(down, start.max(item.created_at), (start + 86_400).min(t))
                            .unwrap_or(100.0)
                    });
                    if up.is_some() {
                        total_down += down;
                    }
                    Day {
                        day: time::day(start),
                        uptime: up,
                        downtime_s: if up.is_some() { down } else { 0 },
                    }
                })
                .collect();
            let state = match states.get(&key) {
                Some(ItemState::Down) => "down",
                Some(ItemState::Issue) => "issue",
                _ if !item.seen => "unknown",
                _ => "up",
            };
            ItemStatus {
                kind: item.kind.as_str(),
                name: item.name.clone(),
                state,
                uptime_90d: Some(
                    uptime(total_down, first_day.max(item.created_at), t).unwrap_or(100.0),
                ),
                days: day_list,
            }
        })
        .collect();

    let overall = if items.iter().any(|i| i.state == "down") {
        "down"
    } else if items.iter().any(|i| i.state == "issue") {
        "issue"
    } else {
        "up"
    };

    let refs: Vec<ItemRef> = public
        .iter()
        .map(|i| ItemRef {
            kind: i.kind,
            id: i.id,
        })
        .collect();
    let name_of = |r: &ItemRef| {
        public
            .iter()
            .find(|p| p.kind == r.kind && p.id == r.id)
            .map(|p| p.name.clone())
            .unwrap_or_default()
    };
    let incidents = history::public(db, project.id, &refs, first_day)
        .await?
        .into_iter()
        .map(|i| IncidentStatus {
            items: i.items.iter().map(name_of).collect(),
            updates: i
                .updates
                .into_iter()
                .map(|u| Update {
                    label: u.label,
                    message: u.message,
                    at: u.at,
                })
                .collect(),
            title: i.title,
            origin: i.origin,
            severity: i.severity,
            started_at: i.started_at,
            resolved_at: i.resolved_at,
        })
        .collect();

    Ok(Some(Status {
        project: ProjectRef {
            name: project.name,
            slug: project.slug,
        },
        overall,
        items,
        incidents,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uptime_rounds_to_two_decimals_and_skips_empty_spans() {
        assert_eq!(uptime(0, 0, 86_400), Some(100.0));
        assert_eq!(uptime(43, 0, 86_400), Some(99.95));
        assert_eq!(uptime(86_400, 0, 86_400), Some(0.0));
        assert_eq!(uptime(0, 100, 100), None);
    }
}
