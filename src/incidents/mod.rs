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

use std::collections::HashMap;

use turso::Connection;

use crate::platform::{caps, db::Db};
use engine::Effect;

pub use manual::{NewIncident, open_manual};
pub use store::{IncidentRow, ItemRef, UpdateRow};

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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Signal {
    CheckOk,
    CheckFailed { cause: String },
    HeartbeatMissed,
    FailPing { cause: String },
    Ping,
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
        /// The source was deleted while down, so "up" would be wrong.
        deleted: bool,
    },
}

fn title_and_cause(source: &Source, sig: &Signal) -> (String, String) {
    match sig {
        Signal::CheckFailed { cause } => (
            format!("{} is down", source.name),
            format!("{cause} after {} checks", engine::FAILS_TO_OPEN),
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

/// Applies one signal for a source. Idempotent for a repeated failure while open.
pub async fn observe(
    c: &Connection,
    source: &Source,
    sig: &Signal,
    now: i64,
) -> anyhow::Result<Option<Event>> {
    let (state, open_id) = store::load_source(c, source.kind, source.id).await?;
    let (next, effect) = engine::step(state, sig);
    let mut opened = open_id;
    let event = match effect {
        None => None,
        Some(Effect::Open) => {
            let (title, cause) = title_and_cause(source, sig);
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
        }
        Some(Effect::Resolve) => match open_id {
            Some(id) => {
                // The opened alert went to the incident's severity; the source's may have changed since.
                let (started, severity) = store::resolve_auto(c, id, None, now).await?;
                opened = None;
                Some(Event::Resolved {
                    incident_id: id,
                    project_id: source.project_id,
                    source_name: source.name.clone(),
                    severity,
                    duration_s: now - started,
                    deleted: false,
                })
            }
            None => {
                opened = None;
                None
            }
        },
    };
    let next_open_id = if next.open { opened } else { None };
    // A passing check on a healthy source changes nothing; skip the write.
    if (next, next_open_id) != (state, open_id) {
        store::save_source(c, source.kind, source.id, next, next_open_id).await?;
    }
    Ok(event)
}

/// A deleted monitor or heartbeat closes its open automatic incident, so nothing stays open forever.
/// Returns the resolved event for the open incident, so whoever got "opened" also hears the end.
pub async fn source_deleted(
    c: &Connection,
    source: &Source,
    now: i64,
) -> anyhow::Result<Option<Event>> {
    let (_, open_id) = store::load_source(c, source.kind, source.id).await?;
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
                deleted: true,
            })
        }
        None => None,
    };
    store::forget_source(c, source.kind, source.id).await?;
    Ok(event)
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

/// Open incidents of a project, for the project's tab bar.
pub async fn open_count(db: &Db, project_id: i64) -> anyhow::Result<i64> {
    store::open_count(&db.read().await?, project_id).await
}

pub async fn list(db: &Db, project_id: i64, open: bool) -> anyhow::Result<Vec<IncidentRow>> {
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

/// The word a status update label shows on the TL pages and the status page.
pub fn label_title(label: &str) -> &'static str {
    match label {
        "investigating" => "Investigating",
        "identified" => "Identified",
        "monitoring" => "Monitoring",
        "resolved" => "Resolved",
        _ => "Update",
    }
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
    #[error("Monita resolves this incident when its check passes.")]
    AutoResolves,
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

/// The labels a TL may post. A closed incident takes follow-up notes as Resolved (AC-19.03); an
/// open automatic incident cannot be marked Resolved, because only a passing check ends it.
pub fn allowed_labels(origin: &str, closed: bool) -> &'static [&'static str] {
    match (closed, origin) {
        (true, _) => &LABELS[3..],
        (false, "auto") => &LABELS[..3],
        _ => &LABELS,
    }
}

/// US-19. A message is required. Resolved closes a manual incident.
pub async fn post_update(
    db: &Db,
    project_id: i64,
    incident_id: i64,
    label: &str,
    message: &str,
    author: i64,
    now: i64,
) -> Result<bool, UpdateError> {
    check_message(message)?;
    let message = message.trim();
    if !LABELS.contains(&label) {
        return Err(UpdateError::BadLabel);
    }
    db.write(async |c| {
        let Some((origin, resolved_at)) = store::origin_and_end(c, project_id, incident_id).await?
        else {
            return Ok(Ok(false));
        };
        if store::count_updates(c, incident_id).await? >= caps::UPDATES_PER_INCIDENT {
            return Ok(Err(UpdateError::TooManyUpdates));
        }
        if !allowed_labels(&origin, resolved_at.is_some()).contains(&label) {
            return Ok(Err(if resolved_at.is_some() {
                UpdateError::Closed
            } else {
                UpdateError::AutoResolves
            }));
        }
        store::add_update(
            c,
            incident_id,
            &origin,
            resolved_at,
            label,
            message,
            Some(author),
            now,
        )
        .await?;
        Ok(Ok(true))
    })
    .await?
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
