//! incidents module: the incident lifecycle and its timeline.
//! Spec: docs/technical-specs/05-module-definitions.md section 5.4 and 12-incident-engine.md.
//! `observe` is the only door for check results. Callers run it inside their own write
//! transaction and pass the returned event to `notify::send` in that same transaction.

pub mod api;
pub mod availability;
pub mod engine;
pub mod history;
mod manual;
mod store;
mod updates;

use std::collections::HashMap;

use turso::Connection;

use crate::platform::{caps, db::Db};
use engine::Effect;

pub use manual::{NewIncident, open_manual};
pub use store::{IncidentRow, ItemRef, UpdateRow};
pub use updates::{allowed_labels, post_update};

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

fn title_and_cause(source: &Source, sig: &Signal) -> (String, String) {
    match sig {
        Signal::CheckFailed { cause } => (
            format!("{} is down", source.name),
            match source.fails_to_open {
                1 => format!("{cause} on 1 check"),
                n => format!("{cause} after {n} checks"),
            },
        ),
        Signal::HeartbeatMissed => (
            format!("{} missed its heartbeat", source.name),
            "No ping within the period and grace time".into(),
        ),
        Signal::FailPing { cause } => {
            (format!("{} reported a failure", source.name), cause.clone())
        }
        Signal::CheckOk | Signal::Ping => (String::new(), String::new()),
    }
}

/// Applies one signal for a source. Idempotent for a repeated failure while down. A source with
/// `auto_incident` off goes down and up without an incident; one with `alert` off returns no event,
/// so nobody is told.
pub async fn observe(
    c: &Connection,
    source: &Source,
    sig: &Signal,
    now: i64,
) -> anyhow::Result<Option<Event>> {
    let (state, open_id, down_since) = store::load_source(c, source.kind, source.id).await?;
    let (next, effect) = engine::step(state, sig, source.fails_to_open);
    let mut opened = open_id;
    let mut since = down_since;
    let event = match effect {
        None => None,
        Some(Effect::Open) => {
            since = Some(now);
            let (title, cause) = title_and_cause(source, sig);
            if source.auto_incident {
                let id = store::open_auto(c, source, &title, &cause, now).await?;
                opened = Some(id);
                Some(Event::Opened {
                    incident_id: id,
                    project_id: source.project_id,
                    source_name: source.name.clone(),
                    title,
                    severity: source.severity.clone(),
                    cause,
                })
            } else {
                Some(Event::Down {
                    project_id: source.project_id,
                    kind: source.kind,
                    source_id: source.id,
                    source_name: source.name.clone(),
                    title,
                    severity: source.severity.clone(),
                    cause,
                })
            }
        }
        Some(Effect::Resolve) => {
            since = None;
            opened = None;
            match (open_id, down_since) {
                (Some(id), _) => {
                    // The opened alert went to the incident's severity; the source's may have changed since.
                    let (started, severity) = store::resolve_auto(c, id, None, now).await?;
                    Some(Event::Resolved {
                        incident_id: id,
                        project_id: source.project_id,
                        source_name: source.name.clone(),
                        severity,
                        duration_s: now - started,
                        ended: Ended::Recovered,
                    })
                }
                (None, Some(at)) => Some(Event::Up {
                    project_id: source.project_id,
                    kind: source.kind,
                    source_id: source.id,
                    source_name: source.name.clone(),
                    severity: source.severity.clone(),
                    duration_s: now - at,
                }),
                (None, None) => None,
            }
        }
    };
    let next_open_id = if next.open { opened } else { None };
    let next_since = if next.open { since } else { None };
    // A passing check on a healthy source changes nothing; skip the write.
    if (next, next_open_id, next_since) != (state, open_id, down_since) {
        store::save_source(c, source.kind, source.id, next, next_open_id, next_since).await?;
    }
    Ok(event.filter(|_| source.alert))
}

/// A deleted monitor or heartbeat closes its open automatic incident, so nothing stays open forever.
/// Returns the resolved event for the open incident, so whoever got "opened" also hears the end.
pub async fn source_deleted(
    c: &Connection,
    source: &Source,
    now: i64,
) -> anyhow::Result<Option<Event>> {
    let (_, open_id, _) = store::load_source(c, source.kind, source.id).await?;
    let event = match open_id {
        Some(id) => {
            let message = "The monitor or heartbeat was deleted.";
            let (started, severity) = store::resolve_auto(c, id, Some(message), now).await?;
            Some(Event::Resolved {
                incident_id: id,
                project_id: source.project_id,
                source_name: source.name.clone(),
                severity,
                duration_s: now - started,
                ended: Ended::Deleted,
            })
        }
        None => None,
    };
    store::forget_source(c, source.kind, source.id).await?;
    Ok(event.filter(|_| source.alert))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemState {
    Up,
    Down,
    Issue,
}

/// Down while an automatic incident is open, Issue while a manual one names the item, else Up.
pub async fn states_for(
    db: &Db,
    project_id: i64,
) -> anyhow::Result<HashMap<(SourceKind, i64), ItemState>> {
    store::states_for(&db.read().await?, project_id).await
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
    Ok(out)
}

/// Open incidents of a project, for the project's tab bar.
pub async fn open_count(db: &Db, project_id: i64) -> anyhow::Result<i64> {
    store::open_count(&db.read().await?, project_id).await
}

/// One page of open incidents across every project (`owner` None) or the projects `owner` owns,
/// critical first, then newest; and how many there are in all.
pub async fn open_across(
    db: &Db,
    owner: Option<i64>,
    page: i64,
) -> anyhow::Result<(Vec<(i64, IncidentRow)>, i64)> {
    let c = db.read().await?;
    let offset = (page.max(1) - 1).saturating_mul(crate::platform::caps::ACROSS_PAGE);
    Ok((
        store::open_across(&c, owner, crate::platform::caps::ACROSS_PAGE, offset).await?,
        store::count_open_across(&c, owner).await?,
    ))
}

/// `open`: Some(true) open only, Some(false) resolved only, None both.
pub async fn list(
    db: &Db,
    project_id: i64,
    open: Option<bool>,
) -> anyhow::Result<Vec<IncidentRow>> {
    store::list(&db.read().await?, project_id, open).await
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
        "{} manual incidents are open, the limit. Resolve one first.",
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
