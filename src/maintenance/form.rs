//! The window form's checks: one for a new or scheduled window, one for an active window's edit.

use super::{MaintenanceError, WindowInput};
use crate::incidents::{ItemRef, SourceKind};

const TITLE_MAX: usize = 120;
pub(super) const MESSAGE_MAX: usize = 1000;
/// A window that silences alerts for longer is more likely a typo than a plan.
pub(super) const LONGEST_S: i64 = 30 * 86_400;

/// The checked form: title, message, start (a past start becomes `now`), end, and the project's
/// items it names. `names` are the project's monitors and heartbeats.
pub(super) fn validate(
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
pub(super) fn validate_active(
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
