//! Status updates a tech lead posts on an incident, and the hand resolve of an automatic one.

use turso::Connection;

use super::{Ended, Event, LABELS, UpdateError, check_message, item_names_on, store};
use crate::platform::{caps, db::Db};

/// The labels a TL may post. A closed incident takes follow-up notes as Resolved (AC-19.03). An
/// open incident takes every label: Resolved closes a manual one, and closes an automatic one by
/// hand, such as when its job was retired on purpose.
pub fn allowed_labels(_origin: &str, closed: bool) -> &'static [&'static str] {
    if closed { &LABELS[3..] } else { &LABELS }
}

/// A tech lead's Resolved on an open automatic incident. The source starts over: a monitor needs
/// two new failed checks to open the next incident, and a heartbeat gets a new deadline from now.
/// Returns the event, so whoever heard "opened" also hears the end; `None` when the source's
/// alerts are off, as they were when it opened.
pub(super) async fn resolve_by_hand(
    c: &Connection,
    project_id: i64,
    incident_id: i64,
    now: i64,
    ended: Ended,
) -> anyhow::Result<Option<Event>> {
    let (started, severity) = store::release_auto(c, incident_id, now).await?;
    let item = store::items(c, incident_id).await?.into_iter().next();
    if let Some(it) = &item
        && it.kind == super::SourceKind::Heartbeat
    {
        crate::checks::rearm_heartbeat(c, it.id, now).await?;
    }
    if let Some(it) = &item
        && !crate::checks::alert_on(c, it.kind, it.id).await?
    {
        return Ok(None);
    }
    let names = item_names_on(c, project_id).await?;
    let source_name = item
        .and_then(|it| names.into_iter().find(|(r, _)| *r == it).map(|(_, n)| n))
        .unwrap_or_else(|| "A deleted monitor or heartbeat".into());
    Ok(Some(Event::Resolved {
        incident_id,
        project_id,
        source_name,
        severity,
        duration_s: now - started,
        ended,
    }))
}

/// US-19. A message is required. Resolved closes the incident; an automatic one alerts its end.
#[allow(clippy::too_many_arguments)]
pub async fn post_update(
    db: &Db,
    base_url: &str,
    project_id: i64,
    incident_id: i64,
    label: &str,
    message: &str,
    author: &super::Author,
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
            return Ok(Err(UpdateError::Closed));
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
        if label == "resolved"
            && origin == "auto"
            && resolved_at.is_none()
            && let Some(ev) =
                resolve_by_hand(c, project_id, incident_id, now, Ended::ByHand).await?
        {
            crate::notify::send(c, &ev, base_url).await?;
        }
        Ok(Ok(true))
    })
    .await?
}
