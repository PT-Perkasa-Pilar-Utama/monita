//! checks module: monitors, heartbeats, pings, results, the scheduler loop.
//! Spec: docs/technical-specs/05-module-definitions.md section 5.3 and 13-check-scheduler.md.

pub mod api;
mod cause;
pub mod chart;
pub mod heartbeat;
mod input;
pub(crate) mod insight;
pub(crate) mod overview;
pub mod ping_routes;
mod probe;
pub mod retention;
pub mod scheduler;
pub mod series;
mod store;

use crate::platform::{caps, db::Db, now, token};

pub use input::{CERT_WARN_DAYS, FieldErrors, MonitorInput, valid_expected_status};
pub use probe::{Auth, Clients, Target, probe, status_matches};
pub use store::{CheckRow, Monitor};

pub const INTERVALS_S: [i64; 4] = [30, 60, 180, 300];

/// Pill kind and word for a monitor or heartbeat. The incident state wins (two failed checks or a
/// missed ping open one); with none open, an item never checked or pinged is waiting.
pub fn item_state(
    seen: bool,
    incident: Option<crate::incidents::ItemState>,
) -> (&'static str, &'static str) {
    match incident {
        Some(crate::incidents::ItemState::Down) => ("down", "Down"),
        Some(crate::incidents::ItemState::Issue) => ("issue", "Issue"),
        Some(crate::incidents::ItemState::Maintenance) => ("maintenance", "Maintenance"),
        Some(crate::incidents::ItemState::Up) => ("up", "Up"),
        None if !seen => ("unknown", "No data yet"),
        None => ("up", "Up"),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckResult {
    pub checked_at: i64,
    pub ok: bool,
    pub status_code: Option<u16>,
    pub latency_ms: i64,
    pub error: Option<String>,
    /// The `notAfter` of the HTTPS server's leaf certificate, when the check reached it.
    pub cert_expires_at: Option<i64>,
}

/// A monitor or heartbeat marked public, for the status page (US-21).
pub struct PublicItem {
    pub kind: crate::incidents::SourceKind,
    pub id: i64,
    pub name: String,
    pub created_at: i64,
    /// False until the first check or ping: the page shows it as waiting.
    pub seen: bool,
}

/// How many monitors and heartbeats a project has, for its tab bar.
pub async fn counts(db: &Db, project_id: i64) -> anyhow::Result<(i64, i64)> {
    let c = db.read().await?;
    Ok((
        store::count_monitors(&c, project_id).await?,
        store::heartbeats::count(&c, project_id).await?,
    ))
}

/// One project-wide line for the Overview: how many there are, the worst state, and a sentence.
#[derive(serde::Serialize)]
pub struct Summary {
    pub total: usize,
    pub kind: &'static str,
    pub text: String,
}

/// The Overview's monitor and heartbeat cards, in that order.
pub async fn summaries(db: &Db, project_id: i64) -> anyhow::Result<[Summary; 2]> {
    use crate::incidents::SourceKind;
    let states = crate::incidents::team_states_for(db, project_id).await?;
    let state = |kind, id| states.get(&(kind, id)).copied();
    let monitors = list_monitors(db, project_id).await?;
    let monitors: Vec<_> = monitors
        .iter()
        .map(|m| item_state(m.last.is_some(), state(SourceKind::Monitor, m.id)).0)
        .collect();
    let beats = heartbeat::list(db, project_id).await?;
    let beats: Vec<_> = beats
        .iter()
        .map(|h| h.state(state(SourceKind::Heartbeat, h.id)).0)
        .collect();
    let line = |states: &[&str], noun| {
        let (kind, text) = overview::summary(states, noun);
        Summary {
            total: states.len(),
            kind,
            text,
        }
    };
    Ok([line(&monitors, "monitor"), line(&beats, "heartbeat")])
}

pub async fn public_items(db: &Db, project_id: i64) -> anyhow::Result<Vec<PublicItem>> {
    store::public_items(&db.read().await?, project_id).await
}

/// AC-11.01: a random point inside the interval, so checks spread out. Never sooner than 5 s, so
/// the TL sees the new monitor as "No data yet" before its first result.
fn first_check_offset(interval_s: i64) -> i64 {
    const FLOOR_S: i64 = 5;
    FLOOR_S + token::random_below((interval_s - FLOOR_S).max(1) as u64) as i64
}

/// What saving a monitor form did, once the form validated.
#[derive(Debug, PartialEq, Eq)]
pub enum Saved {
    /// The monitor's id.
    Done(i64),
    /// The project already has `caps::MONITORS_PER_PROJECT` monitors.
    Full,
    NotFound,
    /// `auth_secret_id` is not a secret of this project.
    ForeignSecret,
}

/// Whether the form's secret belongs to the project, on the writer's connection.
async fn own_secret(
    c: &turso::Connection,
    project_id: i64,
    input: &MonitorInput,
) -> anyhow::Result<bool> {
    match input.auth_secret_id.filter(|_| input.auth_kind() != "none") {
        Some(id) => crate::projects::secrets::exists(c, project_id, id).await,
        None => Ok(true),
    }
}

/// AC-09.01, AC-11.01: the first check lands at a random point inside the interval.
pub async fn create_monitor(
    db: &Db,
    project_id: i64,
    input: &MonitorInput,
) -> anyhow::Result<Saved> {
    let input = &input.for_kind();
    let next = now() + first_check_offset(input.interval_s);
    db.write(async |c| {
        if store::count_monitors(c, project_id).await? >= caps::MONITORS_PER_PROJECT {
            return Ok(Saved::Full);
        }
        if !own_secret(c, project_id, input).await? {
            return Ok(Saved::ForeignSecret);
        }
        store::insert_monitor(c, project_id, input, next)
            .await
            .map(Saved::Done)
    })
    .await
}

/// A changed interval gets a fresh random offset, so edits keep checks spread.
pub async fn update_monitor(
    db: &Db,
    project_id: i64,
    id: i64,
    input: &MonitorInput,
) -> anyhow::Result<Saved> {
    let input = &input.for_kind();
    let next = now() + first_check_offset(input.interval_s);
    db.write(async |c| {
        if !own_secret(c, project_id, input).await? {
            return Ok(Saved::ForeignSecret);
        }
        Ok(
            if store::update_monitor(c, project_id, id, input, next).await? {
                Saved::Done(id)
            } else {
                Saved::NotFound
            },
        )
    })
    .await
}

/// The error of a check whose secret is gone. It names no value.
pub const SECRET_GONE: &str =
    "The secret this monitor signs in with was deleted. Pick one in Edit.";

/// Probes one claimed monitor, with its secret's value when it signs in. `Err` only when the
/// secret could not be read: the check is skipped then, not failed.
pub async fn run_check(db: &Db, clients: &Clients, m: &Monitor) -> anyhow::Result<CheckResult> {
    match m.kind.as_str() {
        "tcp" => return Ok(probe::probe_tcp(&m.url, m.timeout_ms).await),
        "ping" => return Ok(probe::icmp::probe_ping(&m.url, m.timeout_ms).await),
        "dns" => {
            let expected = m.expected_address.as_deref();
            return Ok(probe::probe_dns(&m.url, expected, m.timeout_ms).await);
        }
        _ => {}
    }
    if m.auth_kind == "none" {
        return Ok(probe(&m.target(None), clients).await);
    }
    let secret = match m.auth_secret_id {
        Some(id) => crate::projects::secrets::value_of(&db.read().await?, m.project_id, id).await?,
        None => None,
    };
    Ok(match secret {
        Some(value) => probe(&m.target(Some(&value)), clients).await,
        None => CheckResult {
            checked_at: now(),
            ok: false,
            status_code: None,
            latency_ms: 0,
            error: Some(SECRET_GONE.into()),
            cert_expires_at: None,
        },
    })
}

/// Also closes the monitor's open incident and alerts that it ended.
pub async fn delete_monitor(
    db: &Db,
    base_url: &str,
    project_id: i64,
    id: i64,
) -> anyhow::Result<()> {
    db.write(async |c| {
        if let Some(m) = store::find_monitor(c, project_id, id).await? {
            if let Some(ev) = crate::incidents::source_deleted(c, &m.source(), now()).await? {
                crate::notify::send(c, &ev, base_url).await?;
            }
            store::delete_monitor(c, project_id, id).await?;
        }
        Ok(())
    })
    .await
}

/// Stores one result and feeds it to the incident engine, in one transaction (spec 12).
pub async fn record_check(
    db: &Db,
    base_url: &str,
    monitor: &Monitor,
    result: &CheckResult,
) -> anyhow::Result<()> {
    let signal = if result.ok {
        crate::incidents::Signal::CheckOk
    } else {
        crate::incidents::Signal::CheckFailed {
            cause: result
                .error
                .clone()
                .unwrap_or_else(|| "Check failed".into()),
        }
    };
    db.write(async |c| {
        store::insert_check(c, monitor.id, result).await?;
        if let Some(ev) =
            crate::incidents::observe(c, &monitor.source(), &signal, result.checked_at).await?
        {
            crate::notify::send(c, &ev, base_url).await?;
        }
        if let Some(expires_at) = result.cert_expires_at {
            record_cert(c, base_url, monitor, expires_at).await?;
        }
        Ok(())
    })
    .await
}

/// Keeps the certificate expiry an HTTPS check saw, and warns once per certificate when the
/// monitor asks for it and its alerts are on (v1.1 Q25).
async fn record_cert(
    c: &turso::Connection,
    base_url: &str,
    monitor: &Monitor,
    expires_at: i64,
) -> anyhow::Result<()> {
    let due = monitor.alert
        && probe::cert::warning_due(
            expires_at,
            now(),
            monitor.cert_warn_days,
            monitor.cert_warned_for,
        );
    let warned = due
        && crate::notify::send_cert_warning(
            c,
            &crate::notify::CertWarning {
                project_id: monitor.project_id,
                monitor_id: monitor.id,
                monitor_name: &monitor.name,
                expires_at,
            },
            base_url,
        )
        .await?;
    store::set_cert(c, monitor.id, expires_at, warned).await
}

/// The names of a project's monitors and heartbeats, on the caller's connection, so a writer can
/// read them inside its own transaction.
pub async fn source_names(
    c: &turso::Connection,
    project_id: i64,
) -> anyhow::Result<Vec<(crate::incidents::SourceKind, i64, String)>> {
    store::source_names(c, project_id).await
}

/// After a tech lead resolves a heartbeat's incident by hand: a new deadline from `now`, so a job
/// that stays silent opens the next incident one period plus grace later.
pub async fn rearm_heartbeat(c: &turso::Connection, id: i64, now: i64) -> anyhow::Result<()> {
    store::heartbeats::rearm(c, id, now).await
}

/// How many monitors authenticate with project secret `id`.
pub async fn secret_users(c: &turso::Connection, id: i64) -> anyhow::Result<i64> {
    store::secret_users(c, id).await
}

/// Whether a source's failures alert, on the caller's connection. A deleted source counts as on.
pub async fn alert_on(
    c: &turso::Connection,
    kind: crate::incidents::SourceKind,
    id: i64,
) -> anyhow::Result<bool> {
    store::alert_on(c, kind, id).await
}

pub async fn list_monitors(db: &Db, project_id: i64) -> anyhow::Result<Vec<Monitor>> {
    store::list_monitors(&db.read().await?, project_id).await
}

pub async fn find_monitor(db: &Db, project_id: i64, id: i64) -> anyhow::Result<Option<Monitor>> {
    store::find_monitor(&db.read().await?, project_id, id).await
}

/// AC-10.01: checks of the last 7 days, newest first, at most `limit`, older than `before`.
pub async fn recent_checks(
    db: &Db,
    monitor_id: i64,
    before: Option<(i64, i64)>,
    limit: i64,
) -> anyhow::Result<Vec<CheckRow>> {
    let since = now() - 7 * 86_400;
    store::recent_checks(
        &db.read().await?,
        monitor_id,
        since,
        before.unwrap_or((i64::MAX, i64::MAX)),
        limit,
    )
    .await
}

/// Before its project is deleted, inside the same write: drops the daily summaries of its
/// monitors and heartbeats, which have no foreign key, and returns those sources so the incident
/// engine can forget them too. The project's rows cascade with it.
pub async fn forget_project(
    c: &turso::Connection,
    project_id: i64,
) -> anyhow::Result<Vec<(crate::incidents::SourceKind, i64)>> {
    use crate::incidents::SourceKind;
    let mut gone: Vec<(SourceKind, i64)> = store::list_monitors(c, project_id)
        .await?
        .iter()
        .map(|m| (SourceKind::Monitor, m.id))
        .collect();
    for h in store::heartbeats::list(c, project_id).await? {
        gone.push((SourceKind::Heartbeat, h.id));
    }
    for (kind, id) in &gone {
        store::rollup::delete_summaries(c, *kind, *id).await?;
    }
    Ok(gone)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_check_lands_inside_the_interval_but_not_at_once() {
        for _ in 0..200 {
            let o = first_check_offset(30);
            assert!((5..30).contains(&o), "{o}");
        }
    }
}
