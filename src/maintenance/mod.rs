//! maintenance module: planned work on a project (v1.1 decision Q18, issue #11).
//! While a window is active, a failure of a covered monitor or heartbeat opens no incident and
//! sends no alert; `incidents::observe` asks `covers`. The state follows from the times:
//!
//! | State     | While                   | Leaves by                          | Actor                 |
//! |-----------|-------------------------|------------------------------------|-----------------------|
//! | scheduled | now < starts_at         | time; Delete                       | tech lead, super admin|
//! | active    | starts_at <= now < ends | time; End now (ends_at = now)      | tech lead, super admin|
//! | ended     | ends_at <= now          | never (terminal, read only)        | none                  |
//!
//! Edit (Q50) keeps the state unless the new times change it, and the state is read again inside
//! the write. A scheduled window changes every field. An active one changes its title, message and
//! end only: checks already stored in it carry `in_window`, so its start and items stay. An ended
//! window refuses an edit.

pub mod api;
mod store;

use crate::platform::sql::Connection;
use serde::{Deserialize, Serialize};

use crate::{
    incidents::{ItemRef, SourceKind},
    platform::{caps, db::Db},
};

pub use store::Window;

const TITLE_MAX: usize = 120;
const MESSAGE_MAX: usize = 1000;
/// A window that silences alerts for longer is more likely a typo than a plan.
const LONGEST_S: i64 = 30 * 86_400;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Scheduled,
    Active,
    Ended,
}

pub fn state(starts_at: i64, ends_at: i64, now: i64) -> State {
    if now < starts_at {
        State::Scheduled
    } else if now < ends_at {
        State::Active
    } else {
        State::Ended
    }
}

/// The form. `items` are `kind:id` values, as on the manual incident form.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct WindowInput {
    pub title: String,
    pub message: String,
    pub starts_at: Option<i64>,
    pub ends_at: Option<i64>,
    pub items: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum MaintenanceError {
    /// Every bad field of the form, in the form's order.
    #[error("{}", .0.first().map_or("Fix the fields marked below.", |f| f.1))]
    Invalid(Vec<(&'static str, &'static str)>),
    #[error(
        "{} windows are scheduled or active, which is the limit, so delete or end one first.",
        caps::OPEN_WINDOWS_PER_PROJECT
    )]
    Full,
    #[error("This window has started, so end it instead.")]
    Started,
    #[error("Only an active window can end now.")]
    NotActive,
    #[error("This window has ended, so it stays as a record.")]
    Ended,
    #[error("This window does not exist.")]
    NotFound,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// The checked form: title, message, start (a past start becomes `now`), end, and the project's
/// items it names. `names` are the project's monitors and heartbeats.
fn validate(
    f: &WindowInput,
    names: &[(SourceKind, i64, String)],
    now: i64,
) -> Result<(String, String, i64, i64, Vec<ItemRef>), MaintenanceError> {
    let mut bad = Vec::new();
    let title = check_title(f, &mut bad);
    let start = f.starts_at.map(|s| s.max(now));
    if start.is_none() {
        bad.push(("starts_at", "Pick when the work starts."));
    }
    check_end(start, f.ends_at, now, &mut bad);
    let items: Vec<ItemRef> = names
        .iter()
        .filter(|(kind, id, _)| f.items.contains(&format!("{}:{id}", kind.as_str())))
        .map(|(kind, id, _)| ItemRef {
            kind: *kind,
            id: *id,
        })
        .collect();
    if items.is_empty() {
        bad.push(("items", "Pick at least one monitor or heartbeat."));
    }
    check_message(f, &mut bad);
    match (start, f.ends_at) {
        (Some(s), Some(e)) if bad.is_empty() => {
            Ok((title.into(), f.message.trim().into(), s, e, items))
        }
        _ => Err(MaintenanceError::Invalid(bad)),
    }
}

/// The edit of an active window: its title, message and end against the start it already has.
/// The form's start and items are ignored.
fn validate_active(
    f: &WindowInput,
    start: i64,
    now: i64,
) -> Result<(String, String, i64), MaintenanceError> {
    let mut bad = Vec::new();
    let title = check_title(f, &mut bad);
    check_end(Some(start), f.ends_at, now, &mut bad);
    check_message(f, &mut bad);
    match f.ends_at {
        Some(e) if bad.is_empty() => Ok((title.into(), f.message.trim().into(), e)),
        _ => Err(MaintenanceError::Invalid(bad)),
    }
}

fn check_title<'a>(f: &'a WindowInput, bad: &mut Vec<(&'static str, &'static str)>) -> &'a str {
    let title = f.title.trim();
    if title.is_empty() || title.chars().count() > TITLE_MAX {
        bad.push(("title", "Enter a title of up to 120 characters."));
    }
    title
}

fn check_end(
    start: Option<i64>,
    end: Option<i64>,
    now: i64,
    bad: &mut Vec<(&'static str, &'static str)>,
) {
    match (start, end) {
        (_, None) => bad.push(("ends_at", "Pick when the work ends.")),
        (_, Some(end)) if end <= now => bad.push(("ends_at", "Pick an end in the future.")),
        (Some(s), Some(end)) if end <= s => bad.push(("ends_at", "Pick an end after the start.")),
        (Some(s), Some(end)) if end - s > LONGEST_S => {
            bad.push(("ends_at", "A window lasts 30 days at most."))
        }
        _ => {}
    }
}

fn check_message(f: &WindowInput, bad: &mut Vec<(&'static str, &'static str)>) {
    if f.message.trim().chars().count() > MESSAGE_MAX {
        bad.push(("message", "Use at most 1000 characters."));
    }
}

/// A checked edit: title, message, start, end, and the new items (None keeps them).
type Edit = (String, String, i64, i64, Option<Vec<ItemRef>>);

/// Checks an edit of window `id` against its state at `now` on the caller's connection.
async fn check_edit(
    c: &Connection,
    project_id: i64,
    id: i64,
    f: &WindowInput,
    now: i64,
) -> Result<Edit, MaintenanceError> {
    let Some((s, e)) = store::times(c, project_id, id).await? else {
        return Err(MaintenanceError::NotFound);
    };
    match state(s, e, now) {
        State::Ended => Err(MaintenanceError::Ended),
        State::Active => {
            let (title, message, end) = validate_active(f, s, now)?;
            Ok((title, message, s, end, None))
        }
        State::Scheduled => {
            let names = crate::checks::source_names(c, project_id).await?;
            let (title, message, s, e, items) = validate(f, &names, now)?;
            Ok((title, message, s, e, Some(items)))
        }
    }
}

/// Edits a window (Q50). Checked on a read connection first, so a refused form never takes the
/// writer, then again inside the write, where a scheduled window may have started meanwhile.
pub async fn update(
    db: &Db,
    project_id: i64,
    id: i64,
    f: &WindowInput,
    now: i64,
) -> Result<(), MaintenanceError> {
    check_edit(&db.read().await?, project_id, id, f, now).await?;
    db.write(
        async |c| match check_edit(c, project_id, id, f, now).await {
            Ok((title, message, s, e, items)) => {
                store::update(c, id, &title, &message, s, e, items.as_deref()).await?;
                Ok(Ok(()))
            }
            Err(err) => Ok(Err(err)),
        },
    )
    .await?
}

/// One window of the project, or None.
pub async fn one(db: &Db, project_id: i64, id: i64) -> anyhow::Result<Option<Window>> {
    store::one(&db.read().await?, project_id, id).await
}

/// Plans a window. The form is checked on a read connection first, so a refused form never takes
/// the writer; the items are read again inside the write, so a source deleted meanwhile is skipped.
pub async fn create(
    db: &Db,
    project_id: i64,
    f: &WindowInput,
    now: i64,
) -> Result<i64, MaintenanceError> {
    let read = db.read().await?;
    validate(
        f,
        &crate::checks::source_names(&read, project_id).await?,
        now,
    )?;
    if store::open_count(&read, project_id, now).await? >= caps::OPEN_WINDOWS_PER_PROJECT {
        return Err(MaintenanceError::Full);
    }
    drop(read);
    db.write(async |c| {
        if store::open_count(c, project_id, now).await? >= caps::OPEN_WINDOWS_PER_PROJECT {
            return Ok(Err(MaintenanceError::Full));
        }
        match validate(f, &crate::checks::source_names(c, project_id).await?, now) {
            Ok((title, message, s, e, items)) => Ok(Ok(store::insert(
                c, project_id, &title, &message, s, e, &items, now,
            )
            .await?)),
            Err(err) => Ok(Err(err)),
        }
    })
    .await?
}

/// Deletes a scheduled window. An active one ends instead; an ended one stays as a record.
pub async fn delete(db: &Db, project_id: i64, id: i64, now: i64) -> Result<(), MaintenanceError> {
    db.write(async |c| {
        let Some((s, e)) = store::times(c, project_id, id).await? else {
            return Ok(Err(MaintenanceError::NotFound));
        };
        if state(s, e, now) != State::Scheduled {
            return Ok(Err(MaintenanceError::Started));
        }
        store::delete(c, id).await?;
        Ok(Ok(()))
    })
    .await?
}

/// End now: an active window's end becomes `now`, so the next failure opens an incident.
pub async fn end_now(db: &Db, project_id: i64, id: i64, now: i64) -> Result<(), MaintenanceError> {
    db.write(async |c| {
        let Some((s, e)) = store::times(c, project_id, id).await? else {
            return Ok(Err(MaintenanceError::NotFound));
        };
        if state(s, e, now) != State::Active {
            return Ok(Err(MaintenanceError::NotActive));
        }
        store::set_end(c, id, now).await?;
        Ok(Ok(()))
    })
    .await?
}

/// One page of the project's windows, latest start first, and how many there are.
/// What a page of windows shows: a title search, one state or every state, and the order of
/// their starts.
#[derive(Debug, Default)]
pub struct Filter {
    pub q: String,
    pub state: Option<State>,
    pub oldest: bool,
}

impl State {
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim() {
            "scheduled" => Some(Self::Scheduled),
            "active" => Some(Self::Active),
            "ended" => Some(Self::Ended),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Scheduled => "scheduled",
            Self::Active => "active",
            Self::Ended => "ended",
        }
    }
}

/// One page of the windows that match `f`, and how many match in all.
pub async fn page(
    db: &Db,
    project_id: i64,
    page: i64,
    f: &Filter,
    now: i64,
) -> anyhow::Result<(Vec<Window>, i64)> {
    let c = db.read().await?;
    let offset = (page.max(1) - 1).saturating_mul(caps::WINDOWS_PAGE);
    let state = f.state.map_or("", State::as_str);
    Ok((
        store::page(
            &c,
            project_id,
            &f.q,
            state,
            now,
            f.oldest,
            caps::WINDOWS_PAGE,
            offset,
        )
        .await?,
        store::count(&c, project_id, &f.q, state, now).await?,
    ))
}

/// The end of the latest-ending active window that covers the source, on the caller's
/// connection, or None. `incidents::observe` calls it before it opens an incident.
pub async fn covers(
    c: &Connection,
    kind: SourceKind,
    id: i64,
    at: i64,
) -> anyhow::Result<Option<i64>> {
    store::covers(c, kind, id, at).await
}

/// The active window that covers one source and ends last, for its page's stamp, or None.
pub async fn covering(
    db: &Db,
    project_id: i64,
    kind: SourceKind,
    id: i64,
    at: i64,
) -> anyhow::Result<Option<Window>> {
    let windows = store::active(&db.read().await?, project_id, at).await?;
    Ok(windows
        .into_iter()
        .filter(|w| w.items.iter().any(|i| i.kind == kind && i.id == id))
        .max_by_key(|w| w.ends_at))
}

/// The windows over one source in `[from, to)`, for its chart: (starts_at, ends_at, title).
pub async fn spans(
    c: &Connection,
    kind: SourceKind,
    id: i64,
    from: i64,
    to: i64,
) -> anyhow::Result<Vec<(i64, i64, String)>> {
    store::spans(c, kind, id, from, to).await
}

/// Every monitor and heartbeat an active window covers, for the list rows' tape. An open incident
/// outranks a window in the state, so a covered row can still read Down.
pub async fn covered(
    db: &Db,
    project_id: i64,
    at: i64,
) -> anyhow::Result<std::collections::HashSet<(SourceKind, i64)>> {
    let windows = store::active(&db.read().await?, project_id, at).await?;
    Ok(windows
        .into_iter()
        .flat_map(|w| w.items)
        .map(|i| (i.kind, i.id))
        .collect())
}

/// The project's active windows with their items, at most `caps::OPEN_WINDOWS_PER_PROJECT`.
pub async fn active(c: &Connection, project_id: i64, at: i64) -> anyhow::Result<Vec<Window>> {
    store::active(c, project_id, at).await
}

#[cfg(test)]
mod tests;
