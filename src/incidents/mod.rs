//! incidents module: the incident lifecycle and its timeline.
//! Spec: docs/technical-specs/05-module-definitions.md section 5.4 and 12-incident-engine.md.
//! `observe` is the only door for check results. Callers run it inside their own write
//! transaction and pass the returned event to `notify::send` in that same transaction.

pub mod engine;
pub mod history;
pub mod routes;
mod store;

use std::collections::HashMap;

use turso::Connection;

use crate::platform::{db::Db, time};
use engine::Effect;

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
                let started = store::resolve_auto(c, id, now).await?;
                opened = None;
                Some(Event::Resolved {
                    incident_id: id,
                    project_id: source.project_id,
                    source_name: source.name.clone(),
                    severity: source.severity.clone(),
                    duration_s: now - started,
                })
            }
            None => {
                opened = None;
                None
            }
        },
    };
    store::save_source(
        c,
        source.kind,
        source.id,
        next,
        if next.open { opened } else { None },
    )
    .await?;
    Ok(event)
}

/// A deleted monitor or heartbeat closes its open automatic incident, so nothing stays open forever.
pub async fn source_deleted(
    c: &Connection,
    kind: SourceKind,
    id: i64,
    now: i64,
) -> anyhow::Result<()> {
    let (_, open_id) = store::load_source(c, kind, id).await?;
    if let Some(incident) = open_id {
        store::resolve_with(c, incident, "The monitor or heartbeat was deleted.", now).await?;
    }
    store::forget_source(c, kind, id).await
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemState {
    Up,
    Down,
    Issue,
}

impl ItemState {
    pub fn as_str(self) -> &'static str {
        match self {
            ItemState::Up => "up",
            ItemState::Down => "down",
            ItemState::Issue => "issue",
        }
    }
}

/// Down while an automatic incident is open, Issue while a manual one names the item, else Up.
pub async fn states_for(
    db: &Db,
    project_id: i64,
) -> anyhow::Result<HashMap<(SourceKind, i64), ItemState>> {
    store::states_for(&db.read().await?, project_id).await
}

pub fn duration(started: i64, resolved: Option<i64>, now: i64) -> String {
    time::duration(resolved.unwrap_or(now) - started)
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
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// US-19. A message is required. Resolved closes a manual incident; on an automatic one it is a note.
pub async fn post_update(
    db: &Db,
    project_id: i64,
    incident_id: i64,
    label: &str,
    message: &str,
    author: i64,
    now: i64,
) -> Result<bool, UpdateError> {
    let message = message.trim();
    if message.is_empty() {
        return Err(UpdateError::EmptyMessage);
    }
    if !LABELS.contains(&label) {
        return Err(UpdateError::BadLabel);
    }
    Ok(db
        .write(async |c| {
            store::add_update(
                c,
                project_id,
                incident_id,
                label,
                message,
                Some(author),
                now,
            )
            .await
        })
        .await?)
}

/// US-20. A manual incident starts with an Investigating update.
/// A manual incident as the TL fills it in (US-20).
pub struct NewIncident {
    pub project_id: i64,
    pub title: String,
    pub items: Vec<ItemRef>,
    pub severity: String,
    pub message: String,
    pub author: i64,
}

pub async fn open_manual(db: &Db, n: &NewIncident, now: i64) -> Result<i64, UpdateError> {
    if n.title.trim().is_empty() {
        return Err(UpdateError::EmptyTitle);
    }
    if n.items.is_empty() {
        return Err(UpdateError::NoItems);
    }
    if n.message.trim().is_empty() {
        return Err(UpdateError::EmptyMessage);
    }
    Ok(db
        .write(async |c| store::open_manual(c, n, now).await)
        .await?)
}
