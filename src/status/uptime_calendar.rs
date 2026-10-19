//! The past-uptime page: one public system, three months, a square per day.

use serde::Serialize;

use super::{Day, ProjectRef, months::window};
use crate::{
    checks,
    incidents::{ItemRef, SourceKind, history},
    platform::{db::Db, now, time},
    projects,
};

#[derive(Serialize)]
pub struct System {
    pub kind: &'static str,
    pub id: i64,
    pub name: String,
}

#[derive(Serialize)]
pub struct CalendarMonth {
    /// `2026-10`.
    pub month: String,
    /// Over the days with data; null when the month has none.
    pub uptime: Option<f64>,
    pub days: Vec<Day>,
}

#[derive(Serialize)]
pub struct Calendar {
    pub project: ProjectRef,
    /// Every public system, for the picker.
    pub systems: Vec<System>,
    /// The system shown; null when the project has no public system.
    pub system: Option<System>,
    /// Oldest first; empty without a system.
    pub months: Vec<CalendarMonth>,
    pub prev_to: Option<i64>,
    pub next_to: Option<i64>,
}

/// None when the project does not exist, or `pick` names a system that is not public.
pub async fn calendar(
    db: &Db,
    slug: &str,
    pick: Option<(SourceKind, i64)>,
    to: Option<i64>,
) -> anyhow::Result<Option<Calendar>> {
    let Some(project) = projects::find_public(db, slug).await? else {
        return Ok(None);
    };
    let public = checks::public_items(db, project.id).await?;
    let chosen = match pick {
        Some(key) => match public.iter().find(|i| (i.kind, i.id) == key) {
            Some(item) => Some(item),
            None => return Ok(None),
        },
        None => public.first(),
    };
    let system = |i: &checks::PublicItem| System {
        kind: i.kind.as_str(),
        id: i.id,
        name: i.name.clone(),
    };
    let mut page = Calendar {
        project: ProjectRef {
            name: project.name.clone(),
            slug: project.slug.clone(),
        },
        systems: public.iter().map(system).collect(),
        system: chosen.map(system),
        months: Vec::new(),
        prev_to: None,
        next_to: None,
    };
    let Some(item) = chosen else {
        return Ok(Some(page));
    };
    let t = now();
    let off = crate::platform::settings::utc_offset(&db.read().await?).await?;
    let w = window(to, t, off, item.created_at);
    let spans = history::spans(
        &db.read().await?,
        ItemRef {
            kind: item.kind,
            id: item.id,
        },
        w.starts[0],
        w.end,
        t,
    )
    .await?;
    let ends = [w.starts[1], w.starts[2], w.end];
    for (&start, &end) in w.starts.iter().zip(&ends) {
        let n = ((end - start) / 86_400) as usize;
        let (days, down) = super::day_list(&spans, start, n, item.created_at, t, off);
        let has_data = days.iter().any(|d| d.uptime.is_some());
        page.months.push(CalendarMonth {
            month: time::month_at(start, off),
            uptime: has_data
                .then(|| super::uptime(down, start.max(item.created_at), end.min(t)))
                .flatten(),
            days,
        });
    }
    page.prev_to = w.prev_to;
    page.next_to = w.next_to;
    Ok(Some(page))
}
