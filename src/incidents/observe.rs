//! `observe` and `source_deleted`: check results and deletions into the incident lifecycle.

use turso::Connection;

use super::{Ended, Event, Signal, Source, engine, store};
use engine::Effect;

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
    let (mut next, mut effect) = engine::step(state, sig, source.fails_to_open);
    // Q18: an active maintenance window holds back the open and its alert. A monitor keeps its
    // streak, so its first failed check after the window opens the incident.
    if effect == Some(Effect::Open)
        && crate::maintenance::covers(c, source.kind, source.id, now)
            .await?
            .is_some()
    {
        next.open = false;
        effect = None;
    }
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
