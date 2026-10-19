//! status module: the public status page and its JSON (US-21, US-22).
//! Spec: docs/api-specs/07-status.md. Owns no tables; reads through checks, incidents and projects.

mod months;
pub mod routes;
mod uptime_calendar;

use serde::Serialize;

use crate::{
    checks,
    incidents::{self, ItemRef, ItemState, history},
    platform::{caps, db::Db, now, time},
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
    /// With `kind`, picks the item on the past-uptime page.
    pub id: i64,
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
    /// The history cursor's tie-break; not part of the public JSON.
    #[serde(skip)]
    pub id: i64,
    pub title: String,
    pub origin: String,
    pub severity: String,
    pub started_at: i64,
    pub resolved_at: Option<i64>,
    pub items: Vec<String>,
    pub updates: Vec<Update>,
}

/// The public wire shape of an update. It is its own type on purpose: a field added to the
/// incident side cannot reach the public JSON without an edit here (SEC-21: never the author).
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

/// `days` days from `first_day`, with the item's downtime, and the downtime summed over the days
/// that have data. A day is null when it ended before the item existed or has not begun; an
/// item created a moment ago has had no downtime yet, so its first day reads 100%.
fn day_list(
    spans: &[(i64, i64)],
    first_day: i64,
    days: usize,
    created_at: i64,
    t: i64,
    off: i64,
) -> (Vec<Day>, i64) {
    let mut total_down = 0;
    let list = history::per_day(spans, first_day, days)
        .iter()
        .enumerate()
        .map(|(i, &down)| {
            let start = first_day + i as i64 * 86_400;
            let up = (start + 86_400 > created_at && start <= t).then(|| {
                uptime(down, start.max(created_at), (start + 86_400).min(t)).unwrap_or(100.0)
            });
            if up.is_some() {
                total_down += down;
            }
            Day {
                day: time::day_at(start, off),
                uptime: up,
                downtime_s: if up.is_some() { down } else { 0 },
            }
        })
        .collect();
    (list, total_down)
}

/// The project, its public items with `days` of history, and the overall state.
struct Items {
    project: projects::Project,
    items: Vec<ItemStatus>,
    refs: Vec<(ItemRef, String)>,
    overall: &'static str,
    /// Minutes east of UTC; days start at midnight there.
    off: i64,
}

async fn items(db: &Db, slug: &str, days: usize) -> anyhow::Result<Option<Items>> {
    let Some(project) = projects::find_public(db, slug).await? else {
        return Ok(None);
    };
    let t = now();
    let off = crate::platform::settings::utc_offset(&db.read().await?).await?;
    let first_day = time::day_start_at(t, off) - (days as i64 - 1) * 86_400;
    let public = checks::public_items(db, project.id).await?;
    let states = incidents::states_for(db, project.id).await?;
    let intervals = history::intervals(db, project.id, first_day, t).await?;

    let items: Vec<ItemStatus> = public
        .iter()
        .map(|item| {
            let key = (item.kind, item.id);
            let spans = intervals.get(&key).map(Vec::as_slice).unwrap_or(&[]);
            let (day_rows, total_down) = day_list(spans, first_day, days, item.created_at, t, off);
            let state = match states.get(&key) {
                Some(ItemState::Down) => "down",
                Some(ItemState::Issue) => "issue",
                _ if !item.seen => "unknown",
                _ => "up",
            };
            ItemStatus {
                kind: item.kind.as_str(),
                id: item.id,
                name: item.name.clone(),
                state,
                uptime_90d: Some(
                    uptime(total_down, first_day.max(item.created_at), t).unwrap_or(100.0),
                ),
                days: day_rows,
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

    let refs = public
        .iter()
        .map(|i| {
            (
                ItemRef {
                    kind: i.kind,
                    id: i.id,
                },
                i.name.clone(),
            )
        })
        .collect();
    Ok(Some(Items {
        project,
        items,
        refs,
        overall,
        off,
    }))
}

/// Public incidents of `which`, as wire types: public item names, no author, at most `limit`.
/// None when the project turned its Incidents tab off.
async fn incidents(
    db: &Db,
    it: &Items,
    which: history::Touching,
    limit: i64,
) -> anyhow::Result<Vec<IncidentStatus>> {
    if !it.project.pages.incidents {
        return Ok(Vec::new());
    }
    let refs: Vec<ItemRef> = it.refs.iter().map(|(r, _)| r.clone()).collect();
    let name_of = |r: &ItemRef| {
        it.refs
            .iter()
            .find(|(p, _)| p.kind == r.kind && p.id == r.id)
            .map(|(_, n)| n.clone())
            .unwrap_or_default()
    };
    Ok(history::public(db, it.project.id, &refs, which, limit)
        .await?
        .into_iter()
        .map(|i| IncidentStatus {
            id: i.id,
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
        .collect())
}

fn status(it: Items, incidents: Vec<IncidentStatus>) -> Status {
    Status {
        project: ProjectRef {
            name: it.project.name,
            slug: it.project.slug,
        },
        overall: it.overall,
        items: it.items,
        incidents,
    }
}

/// The JSON: items over the last `days` days in the admin's offset, and incidents started in
/// them or still open.
pub async fn load(db: &Db, slug: &str, days: usize) -> anyhow::Result<Option<Status>> {
    let Some(it) = items(db, slug, days).await? else {
        return Ok(None);
    };
    let first_day = time::day_start_at(now(), it.off) - (days as i64 - 1) * 86_400;
    let list = incidents(
        db,
        &it,
        history::Touching::Window(first_day),
        caps::PUBLIC_INCIDENTS,
    )
    .await?;
    Ok(Some(status(it, list)))
}

/// The page: 90 days of items, every open incident, and the newest resolved ones of the last
/// 7 days. Older incidents live on the history page.
pub struct PageData {
    pub status: Status,
    pub recent: Vec<IncidentStatus>,
}

pub async fn page(db: &Db, slug: &str) -> anyhow::Result<Option<PageData>> {
    let Some(it) = items(db, slug, 90).await? else {
        return Ok(None);
    };
    let open = incidents(db, &it, history::Touching::Open, caps::OPEN_INCIDENTS).await?;
    let week = history::Touching::ResolvedSince(now() - 7 * 86_400);
    let recent = incidents(db, &it, week, caps::PUBLIC_RECENT).await?;
    Ok(Some(PageData {
        status: status(it, open),
        recent,
    }))
}

/// One page of the incident history, older than the `(started_at, id)` cursor. None also when
/// the project turned its Incidents tab off.
pub async fn history_page(
    db: &Db,
    slug: &str,
    before: Option<(i64, i64)>,
) -> anyhow::Result<Option<(ProjectRef, Vec<IncidentStatus>)>> {
    let Some(it) = items(db, slug, 1)
        .await?
        .filter(|it| it.project.pages.incidents)
    else {
        return Ok(None);
    };
    let list = incidents(
        db,
        &it,
        history::Touching::Before(before),
        caps::PUBLIC_HISTORY_PAGE,
    )
    .await?;
    let project = ProjectRef {
        name: it.project.name,
        slug: it.project.slug,
    };
    Ok(Some((project, list)))
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
