//! maintenance module: planned work on a project (v1.1 decision Q18, issue #11).
//! While a window is active, a failure of a covered monitor or heartbeat opens no incident and
//! sends no alert; `incidents::observe` asks `covers`. The state follows from the times:
//!
//! | State     | While                   | Leaves by                          | Actor                 |
//! |-----------|-------------------------|------------------------------------|-----------------------|
//! | scheduled | now < starts_at         | time; Delete                       | tech lead, super admin|
//! | active    | starts_at <= now < ends | time; End now (ends_at = now)      | tech lead, super admin|
//! | ended     | ends_at <= now          | never (terminal, read only)        | none                  |

pub mod api;
mod store;

use serde::{Deserialize, Serialize};
use turso::Connection;

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
    let title = f.title.trim();
    if title.is_empty() || title.chars().count() > TITLE_MAX {
        bad.push(("title", "Enter a title of up to 120 characters."));
    }
    let start = f.starts_at.map(|s| s.max(now));
    if start.is_none() {
        bad.push(("starts_at", "Pick when the work starts."));
    }
    match (start, f.ends_at) {
        (_, None) => bad.push(("ends_at", "Pick when the work ends.")),
        (_, Some(end)) if end <= now => bad.push(("ends_at", "Pick an end in the future.")),
        (Some(s), Some(end)) if end <= s => bad.push(("ends_at", "Pick an end after the start.")),
        (Some(s), Some(end)) if end - s > LONGEST_S => {
            bad.push(("ends_at", "A window lasts 30 days at most."))
        }
        _ => {}
    }
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
    if f.message.trim().chars().count() > MESSAGE_MAX {
        bad.push(("message", "Use at most 1000 characters."));
    }
    match (start, f.ends_at) {
        (Some(s), Some(e)) if bad.is_empty() => {
            Ok((title.into(), f.message.trim().into(), s, e, items))
        }
        _ => Err(MaintenanceError::Invalid(bad)),
    }
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
pub async fn page(db: &Db, project_id: i64, page: i64) -> anyhow::Result<(Vec<Window>, i64)> {
    let c = db.read().await?;
    let offset = (page.max(1) - 1).saturating_mul(caps::WINDOWS_PAGE);
    Ok((
        store::page(&c, project_id, caps::WINDOWS_PAGE, offset).await?,
        store::count(&c, project_id).await?,
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

/// The project's active windows with their items, at most `caps::OPEN_WINDOWS_PER_PROJECT`.
pub async fn active(c: &Connection, project_id: i64, at: i64) -> anyhow::Result<Vec<Window>> {
    store::active(c, project_id, at).await
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_000_000;

    fn names() -> Vec<(SourceKind, i64, String)> {
        vec![
            (SourceKind::Monitor, 1, "api".into()),
            (SourceKind::Heartbeat, 2, "nightly".into()),
        ]
    }

    fn form() -> WindowInput {
        WindowInput {
            title: "Database upgrade".into(),
            starts_at: Some(NOW + 60),
            ends_at: Some(NOW + 3600),
            items: vec!["monitor:1".into(), "monitor:9".into()],
            ..WindowInput::default()
        }
    }

    fn fields(f: &WindowInput) -> Vec<&'static str> {
        match validate(f, &names(), NOW) {
            Err(MaintenanceError::Invalid(bad)) => bad.into_iter().map(|b| b.0).collect(),
            other => panic!("expected field errors, got {other:?}"),
        }
    }

    #[test]
    fn the_state_follows_from_the_times() {
        assert_eq!(state(10, 20, 9), State::Scheduled);
        assert_eq!(state(10, 20, 10), State::Active);
        assert_eq!(state(10, 20, 19), State::Active);
        assert_eq!(state(10, 20, 20), State::Ended);
    }

    #[test]
    fn a_good_form_keeps_only_the_projects_items_and_a_past_start_starts_now() {
        let (_, _, s, _, items) = validate(&form(), &names(), NOW).unwrap();
        assert_eq!(s, NOW + 60);
        assert_eq!(items.len(), 1, "monitor 9 is not in the project");
        let past = WindowInput {
            starts_at: Some(NOW - 600),
            ..form()
        };
        assert_eq!(validate(&past, &names(), NOW).unwrap().2, NOW);
    }

    #[test]
    fn a_bad_form_names_every_bad_field() {
        let empty = WindowInput::default();
        assert_eq!(fields(&empty), ["title", "starts_at", "ends_at", "items"]);
        let ended = WindowInput {
            ends_at: Some(NOW),
            ..form()
        };
        assert_eq!(fields(&ended), ["ends_at"]);
        let backwards = WindowInput {
            starts_at: Some(NOW + 7200),
            ..form()
        };
        assert_eq!(fields(&backwards), ["ends_at"]);
        let long = WindowInput {
            ends_at: Some(NOW + 60 + LONGEST_S + 1),
            ..form()
        };
        assert_eq!(fields(&long), ["ends_at"]);
        let loud = WindowInput {
            message: "x".repeat(MESSAGE_MAX + 1),
            ..form()
        };
        assert_eq!(fields(&loud), ["message"]);
    }
}
