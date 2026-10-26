//! incidents module: the incident lifecycle and its timeline.
//! Spec: docs/technical-specs/05-module-definitions.md section 5.4 and 12-incident-engine.md.
//! `observe` is the only door for check results. Callers run it inside their own write
//! transaction and pass the returned event to `notify::send` in that same transaction.

pub mod api;
pub mod availability;
pub mod engine;
pub mod history;
mod manual;
mod observe;
mod reclassify;
mod store;
mod updates;

use std::collections::HashMap;

use crate::platform::sql::Connection;

use crate::platform::{caps, db::Db};

pub use manual::{NewIncident, open_manual};
pub use observe::{observe, source_deleted, source_paused};
pub use reclassify::{Class, reclassify};
pub use store::{IncidentRow, ItemRef, UpdateRow};
pub use updates::{allowed_labels, post_update};

/// Who wrote a status update or opened a manual incident: a signed-in user, or a project API key
/// by its name (v1.1 Q44). An update with no author is Monita's own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Author {
    User(i64),
    Key(String),
}

impl Author {
    fn user_id(&self) -> Option<i64> {
        match self {
            Author::User(id) => Some(*id),
            Author::Key(_) => None,
        }
    }

    fn key_name(&self) -> Option<&str> {
        match self {
            Author::User(_) => None,
            Author::Key(name) => Some(name),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SourceKind {
    Monitor,
    Heartbeat,
}

impl SourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            SourceKind::Monitor => "monitor",
            SourceKind::Heartbeat => "heartbeat",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "monitor" => Some(SourceKind::Monitor),
            "heartbeat" => Some(SourceKind::Heartbeat),
            _ => None,
        }
    }
}

/// What the engine needs to know about a monitor or heartbeat. The checks module fills it,
/// so this module never reads checks tables.
#[derive(Clone, Debug)]
pub struct Source {
    pub kind: SourceKind,
    pub id: i64,
    pub project_id: i64,
    pub name: String,
    pub severity: String,
    /// Its failures alert. Off: Monita tracks it and tells nobody.
    pub alert: bool,
    /// Its failures open an incident. Off: it goes Down on the team's pages and alerts only.
    pub auto_incident: bool,
    /// Failed checks in a row that count as down; a heartbeat ignores it.
    pub fails_to_open: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Signal {
    CheckOk,
    CheckFailed { cause: String },
    HeartbeatMissed,
    FailPing { cause: String },
    Ping,
}

/// How an automatic incident ended. Only Recovered means the source works again.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ended {
    Recovered,
    Deleted,
    ByHand,
    /// Reclassified as planned work, or as a false alarm (v1.1 Q46).
    Planned,
    FalseAlarm,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    Opened {
        incident_id: i64,
        project_id: i64,
        source_name: String,
        title: String,
        severity: String,
        cause: String,
    },
    Resolved {
        incident_id: i64,
        project_id: i64,
        source_name: String,
        severity: String,
        duration_s: i64,
        /// Why it ended: a passing check, a deleted source, or a tech lead's Resolved update.
        ended: Ended,
    },
    /// A source went down and does not open incidents: the alert alone tells the team.
    Down {
        project_id: i64,
        kind: SourceKind,
        source_id: i64,
        source_name: String,
        title: String,
        severity: String,
        cause: String,
    },
    /// That source works again.
    Up {
        project_id: i64,
        kind: SourceKind,
        source_id: i64,
        source_name: String,
        severity: String,
        duration_s: i64,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemState {
    Up,
    Down,
    Issue,
    /// An active maintenance window covers it, and no incident names it.
    Maintenance,
}

/// Down while an automatic incident is open, Issue while a manual one names the item,
/// Maintenance while an active window covers it, else Up.
pub async fn states_for(
    db: &Db,
    project_id: i64,
) -> anyhow::Result<HashMap<(SourceKind, i64), ItemState>> {
    let c = db.read().await?;
    let mut out = store::states_for(&c, project_id).await?;
    add_maintenance(&c, project_id, &mut out).await?;
    Ok(out)
}

/// An incident outranks a window: one that was open when the window began still shows.
async fn add_maintenance(
    c: &Connection,
    project_id: i64,
    out: &mut HashMap<(SourceKind, i64), ItemState>,
) -> anyhow::Result<()> {
    for w in crate::maintenance::active(c, project_id, crate::platform::now()).await? {
        for r in w.items {
            out.entry((r.kind, r.id)).or_insert(ItemState::Maintenance);
        }
    }
    Ok(())
}

/// What the team's own pages show: `states_for`, plus Down for a source that is down with no
/// incident because its auto-incident is off. The public status page keeps `states_for`, so only
/// an incident someone opened reaches visitors.
pub async fn team_states_for(
    db: &Db,
    project_id: i64,
) -> anyhow::Result<HashMap<(SourceKind, i64), ItemState>> {
    let c = db.read().await?;
    let mut out = store::states_for(&c, project_id).await?;
    let refs: Vec<ItemRef> = item_names_on(&c, project_id)
        .await?
        .into_iter()
        .map(|(r, _)| r)
        .collect();
    for r in store::down_without_incident(&c, &refs).await? {
        out.insert((r.kind, r.id), ItemState::Down);
    }
    add_maintenance(&c, project_id, &mut out).await?;
    Ok(out)
}

/// The projects that are down and the projects with an issue, as their cards show them: down
/// when a source has an open automatic incident or is down with its incident switch off; an issue
/// when a source is in an open manual incident and none is down. For the project list's filter.
pub async fn troubled_projects(db: &Db) -> anyhow::Result<(Vec<i64>, Vec<i64>)> {
    let c = db.read().await?;
    let open = store::open_item_projects(&c).await?;
    let quiet = store::down_sources_without_incident(&c).await?;
    let mut down: Vec<i64> = open
        .iter()
        .filter(|(_, auto)| *auto)
        .map(|(p, _)| *p)
        .collect();
    down.extend(crate::checks::projects_of(&c, &quiet).await?);
    down.sort_unstable();
    down.dedup();
    let issue = open
        .iter()
        .filter(|(p, auto)| !auto && down.binary_search(p).is_err())
        .map(|(p, _)| *p)
        .collect();
    Ok((down, issue))
}

/// Open incidents of a project, for the project's tab bar.
pub async fn open_count(db: &Db, project_id: i64) -> anyhow::Result<i64> {
    store::open_count(&db.read().await?, project_id).await
}

pub use store::AcrossFilter;

/// One page of incidents across every project (`owner` None) or the projects `owner` owns, with
/// the page's search, filters and sort; and how many match in all.
pub async fn across(
    db: &Db,
    owner: Option<i64>,
    f: &AcrossFilter<'_>,
    page: i64,
) -> anyhow::Result<(Vec<(i64, IncidentRow)>, i64)> {
    let c = db.read().await?;
    let offset = (page.max(1) - 1).saturating_mul(crate::platform::caps::ACROSS_PAGE);
    Ok((
        store::across(&c, owner, f, crate::platform::caps::ACROSS_PAGE, offset).await?,
        store::count_across(&c, owner, f).await?,
    ))
}

/// `open`: Some(true) open only, Some(false) resolved only, None both.
/// The project's newest 200 incidents: open, resolved or both, and open at some time in `range`.
pub async fn list(
    db: &Db,
    project_id: i64,
    open: Option<bool>,
    range: (Option<i64>, Option<i64>),
) -> anyhow::Result<Vec<IncidentRow>> {
    store::list(&db.read().await?, project_id, open, range).await
}

pub async fn find(
    db: &Db,
    project_id: i64,
    id: i64,
) -> anyhow::Result<Option<(IncidentRow, Vec<ItemRef>, Vec<UpdateRow>)>> {
    let conn = db.read().await?;
    let Some(incident) = store::find(&conn, project_id, id).await? else {
        return Ok(None);
    };
    let items = store::items(&conn, id).await?;
    let updates = store::updates(&conn, id).await?;
    Ok(Some((incident, items, updates)))
}

pub const LABELS: [&str; 4] = ["investigating", "identified", "monitoring", "resolved"];

#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    #[error("Write a message.")]
    EmptyMessage,
    #[error("Pick a label.")]
    BadLabel,
    #[error("Enter a title.")]
    EmptyTitle,
    #[error("Pick at least one monitor or heartbeat.")]
    NoItems,
    #[error("The title needs at most 120 characters.")]
    TitleTooLong,
    #[error("The message needs at most 10 000 characters.")]
    MessageTooLong,
    #[error("Pick critical or warning.")]
    BadSeverity,
    #[error("A closed incident takes only Resolved updates.")]
    Closed,
    #[error("An incident takes {} updates at most.", caps::UPDATES_PER_INCIDENT)]
    TooManyUpdates,
    #[error(
        "{} manual incidents are open, which is the limit, so resolve one first.",
        caps::OPEN_MANUAL_PER_PROJECT
    )]
    TooManyOpen,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

const MAX_TITLE: usize = 120;
const MAX_MESSAGE: usize = 10_000;

/// Titles and messages reach the public status page, so they get the ticket limits.
fn check_message(message: &str) -> Result<(), UpdateError> {
    let n = message.trim().chars().count();
    if n == 0 {
        Err(UpdateError::EmptyMessage)
    } else if n > MAX_MESSAGE {
        Err(UpdateError::MessageTooLong)
    } else {
        Ok(())
    }
}

/// Names of the project's monitors and heartbeats, for item lists and the manual form. Read
/// through `checks`, so this module never reads its tables.
pub async fn item_names(db: &Db, project_id: i64) -> anyhow::Result<Vec<(ItemRef, String)>> {
    item_names_on(&db.read().await?, project_id).await
}

async fn item_names_on(c: &Connection, project_id: i64) -> anyhow::Result<Vec<(ItemRef, String)>> {
    Ok(crate::checks::source_names(c, project_id)
        .await?
        .into_iter()
        .map(|(kind, id, name)| (ItemRef { kind, id }, name))
        .collect())
}

/// Before a project is deleted, inside the same write: forgets its sources' fail streaks. Its
/// incidents cascade with the project, so no alert goes out.
pub async fn forget_sources(c: &Connection, sources: &[(SourceKind, i64)]) -> anyhow::Result<()> {
    for (kind, id) in sources {
        store::forget_source(c, *kind, *id).await?;
    }
    Ok(())
}
