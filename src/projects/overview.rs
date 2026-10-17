//! A project's Overview: where its monitors, heartbeats, tickets and incidents stand, and its facts.

use askama::Template;
use axum::{
    extract::{Path, State},
    response::Response,
};

use super::Project;
use crate::{
    accounts::CurrentUser,
    app::AppState,
    checks::{self, Summary},
    incidents::{self, IncidentRow},
    platform::{
        now,
        page::{PageError, Shell},
        render::render,
        time,
    },
    tickets,
};

/// Open incidents shown on the page; the Incidents tab lists the rest.
const INCIDENTS: usize = 5;

#[derive(Template)]
#[template(path = "projects/overview.html")]
struct Overview {
    shell: Shell,
    project: Project,
    monitors: Summary,
    heartbeats: Summary,
    triage: i64,
    open_tickets: i64,
    /// The newest open incidents, at most `INCIDENTS`.
    incidents: Vec<IncidentRow>,
    open_incidents: i64,
    active_keys: usize,
    now: i64,
}

impl Overview {
    fn initial(&self) -> char {
        self.project
            .name
            .chars()
            .next()
            .unwrap_or('?')
            .to_ascii_uppercase()
    }
    fn created(&self) -> String {
        time::utc(self.project.created_at)
    }
    fn started(&self, i: &IncidentRow) -> String {
        time::ago(i.started_at, self.now)
    }
    fn duration(&self, i: &IncidentRow) -> String {
        incidents::duration(i.started_at, i.resolved_at, self.now)
    }
    fn label(&self, i: &IncidentRow) -> &'static str {
        incidents::label_title(i.last_label.as_deref().unwrap_or(""))
    }
    fn plural(&self, n: &i64, noun: &str) -> String {
        format!("{n} {noun}{}", if *n == 1 { "" } else { "s" })
    }
}

pub async fn page(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
) -> Result<Response, PageError> {
    let project = super::find_for(&state.db, &user, &slug).await?;
    let [monitors, heartbeats] = checks::summaries(&state.db, project.id).await?;
    let stages = tickets::stage_counts(&state.db, project.id).await?;
    let stage = |s: &str| stages.iter().find(|(k, _)| k == s).map_or(0, |(_, n)| *n);
    let mut open = incidents::list(&state.db, project.id, true).await?;
    open.truncate(INCIDENTS);
    let keys = super::list_keys(&state.db, project.id).await?;
    let shell = super::shell(&state.db, &user, "overview", Some(&project)).await?;
    Ok(render(&Overview {
        open_tickets: shell.counts.tickets,
        open_incidents: shell.counts.incidents,
        shell,
        monitors,
        heartbeats,
        triage: stage("triage"),
        incidents: open,
        active_keys: keys.iter().filter(|k| k.revoked_at.is_none()).count(),
        project,
        now: now(),
    }))
}
