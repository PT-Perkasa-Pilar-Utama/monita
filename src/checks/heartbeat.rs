//! Heartbeats: a job pings Monita; a missing ping is the failure. Spec 13 section 13.1.

use super::{FieldErrors, store::heartbeats as store};
use crate::platform::{
    caps,
    db::Db,
    now,
    rate_limit::{self, Limiter, Verdict},
    settings, token,
};
use crate::{
    incidents::{self, Signal, Source, SourceKind},
    maintenance, notify,
};

pub const PERIODS_S: [(i64, &str); 12] = [
    (60, "1 minute"),
    (300, "5 minutes"),
    (600, "10 minutes"),
    (900, "15 minutes"),
    (1_800, "30 minutes"),
    (3_600, "1 hour"),
    (7_200, "2 hours"),
    (21_600, "6 hours"),
    (43_200, "12 hours"),
    (86_400, "1 day"),
    (604_800, "7 days"),
    (2_592_000, "30 days"),
];

pub const GRACES_S: [(i64, &str); 9] = [
    (0, "None"),
    (60, "1 minute"),
    (300, "5 minutes"),
    (900, "15 minutes"),
    (1_800, "30 minutes"),
    (3_600, "1 hour"),
    (7_200, "2 hours"),
    (21_600, "6 hours"),
    (86_400, "1 day"),
];

pub fn label(table: &[(i64, &'static str)], s: i64) -> String {
    table
        .iter()
        .find(|(v, _)| *v == s)
        .map(|(_, l)| l.to_string())
        .unwrap_or_else(|| format!("{s}s"))
}

pub struct Heartbeat {
    pub id: i64,
    pub project_id: i64,
    pub name: String,
    pub token: String,
    pub period_s: i64,
    pub grace_s: i64,
    pub severity: String,
    pub is_public: bool,
    pub alert: bool,
    pub auto_incident: bool,
    pub last_ping_at: Option<i64>,
    pub deadline_at: Option<i64>,
    pub created_at: i64,
}

impl Heartbeat {
    pub fn state(&self, incident: Option<incidents::ItemState>) -> (&'static str, &'static str) {
        super::item_state(self.last_ping_at.is_some(), incident)
    }

    pub fn source(&self) -> Source {
        Source {
            kind: SourceKind::Heartbeat,
            id: self.id,
            project_id: self.project_id,
            name: self.name.clone(),
            severity: self.severity.clone(),
            alert: self.alert,
            auto_incident: self.auto_incident,
            fails_to_open: 1,
        }
    }

    pub fn input(&self) -> HeartbeatInput {
        HeartbeatInput {
            name: self.name.clone(),
            period_s: self.period_s,
            grace_s: self.grace_s,
            severity: self.severity.clone(),
            is_public: self.is_public,
            alert: self.alert,
            auto_incident: self.auto_incident,
        }
    }
}

#[derive(Clone, Debug)]
pub struct HeartbeatInput {
    pub name: String,
    pub period_s: i64,
    pub grace_s: i64,
    pub severity: String,
    pub is_public: bool,
    /// A missed deadline or a fail ping alerts the severity's recipients.
    pub alert: bool,
    /// A missed deadline or a fail ping opens an incident on its own.
    pub auto_incident: bool,
}

impl Default for HeartbeatInput {
    fn default() -> Self {
        Self {
            name: String::new(),
            period_s: 86_400,
            grace_s: 1_800,
            severity: "warning".into(),
            is_public: false,
            alert: true,
            auto_incident: true,
        }
    }
}

impl HeartbeatInput {
    pub fn validate(&self) -> FieldErrors {
        let mut e = FieldErrors::new();
        if self.name.trim().is_empty() || self.name.chars().count() > 80 {
            e.insert("name", "Enter a name of up to 80 characters.");
        }
        if !PERIODS_S.iter().any(|(v, _)| *v == self.period_s) {
            e.insert("period_s", "Pick one of the listed periods.");
        }
        if !GRACES_S.iter().any(|(v, _)| *v == self.grace_s) {
            e.insert("grace_s", "Pick how late the job can be from the list.");
        }
        if !matches!(self.severity.as_str(), "critical" | "warning") {
            e.insert("severity", "Pick critical or warning.");
        }
        e
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PingKind {
    Ok,
    Fail,
}

#[derive(serde::Serialize)]
pub struct PingRow {
    pub id: i64,
    pub received_at: i64,
    pub kind: String,
    pub source_ip: Option<String>,
    /// What a fail ping reported: the first 1 KB of its body.
    pub cause: Option<String>,
}

/// AC-12.01: a heartbeat that never pings still misses its first deadline. `None` when the
/// project already has `caps::HEARTBEATS_PER_PROJECT` heartbeats.
pub async fn create(
    db: &Db,
    project_id: i64,
    input: &HeartbeatInput,
) -> anyhow::Result<Option<i64>> {
    let tok = token::random_token();
    let deadline = now() + input.period_s + input.grace_s;
    db.write(async |c| {
        if store::count(c, project_id).await? >= caps::HEARTBEATS_PER_PROJECT {
            return Ok(None);
        }
        store::insert(c, project_id, input, &tok, deadline)
            .await
            .map(Some)
    })
    .await
}

/// A pending deadline is recomputed from the last ping with the new period and grace.
pub async fn update(
    db: &Db,
    project_id: i64,
    id: i64,
    input: &HeartbeatInput,
) -> anyhow::Result<bool> {
    db.write(async |c| store::update(c, project_id, id, input).await)
        .await
}

/// Also closes the heartbeat's open incident and alerts that it ended.
pub async fn delete(db: &Db, base_url: &str, project_id: i64, id: i64) -> anyhow::Result<()> {
    db.write(async |c| {
        if let Some(hb) = store::find(c, project_id, id).await? {
            if let Some(ev) = incidents::source_deleted(c, &hb.source(), now()).await? {
                notify::send(c, &ev, base_url).await?;
            }
            store::delete(c, project_id, id).await?;
        }
        Ok(())
    })
    .await
}

/// The newest ping of a heartbeat, good or failed. `last_ping_at` counts good pings only,
/// because only they move the deadline.
#[derive(Clone, Debug, serde::Serialize)]
pub struct LastPing {
    pub ok: bool,
    /// What a fail ping reported; `None` for a good ping.
    pub cause: Option<String>,
    pub at: i64,
}

/// The newest ping of each heartbeat in the project, by heartbeat id.
pub async fn last_pings(
    db: &Db,
    project_id: i64,
) -> anyhow::Result<std::collections::HashMap<i64, LastPing>> {
    Ok(store::last_pings(&db.read().await?, project_id)
        .await?
        .into_iter()
        .collect())
}

pub async fn list(db: &Db, project_id: i64) -> anyhow::Result<Vec<Heartbeat>> {
    store::list(&db.read().await?, project_id).await
}

pub async fn find(db: &Db, project_id: i64, id: i64) -> anyhow::Result<Option<Heartbeat>> {
    store::find(&db.read().await?, project_id, id).await
}

/// One page of the last 7 days' pings, newest first, older than `before`.
pub async fn recent_pings(
    db: &Db,
    id: i64,
    before: Option<(i64, i64)>,
) -> anyhow::Result<Vec<PingRow>> {
    let since = now() - 7 * 86_400;
    store::recent_pings(&db.read().await?, id, since, before, caps::PINGS_PAGE).await
}

/// Proof that `admit` passed. Only `admit` makes one and only `record` takes it, so no caller can
/// store a ping that skipped the token and the limits.
pub struct Admitted {
    /// The token only: `record` reads the heartbeat again, so nothing here can be stale. A
    /// reset can reuse an id; it never reuses a token.
    token: String,
    kind: PingKind,
    source_ip: Option<String>,
}

pub enum Admission {
    Admitted(Admitted),
    UnknownToken,
    /// Seconds until the next ping is accepted.
    Limited(i64),
}

pub struct Ping<'a> {
    pub token: &'a str,
    pub kind: PingKind,
    pub source_ip: Option<std::net::IpAddr>,
}

/// Unknown-token pings allowed per minute from one IP before 429. A real job never misses;
/// a scanner does, and each miss costs a lookup.
const MISSES_PER_MIN: u32 = 60;

/// The checks before a ping is read or stored: the token, then the limits. AC-13.02: over
/// `rl_ping_per_s` a ping is refused, and the ping already counted this second keeps the
/// heartbeat alive. `/fail` has its own bucket, so a job that pings and then fails in one second
/// is still heard. A caller reads a ping body only after `Admitted`.
pub async fn admit(db: &Db, limiter: &Limiter, p: &Ping<'_>) -> anyhow::Result<Admission> {
    let miss_key = format!("pingmiss:{}", rate_limit::ip_key(p.source_ip));
    if let Verdict::Deny(retry) = limiter.over(&miss_key, MISSES_PER_MIN, 60, now()) {
        return Ok(Admission::Limited(retry));
    }
    let (hb, limit) = {
        let conn = db.read().await?;
        let Some(hb) = store::find_by_token(&conn, p.token).await? else {
            limiter.hit(&miss_key, MISSES_PER_MIN, 60, now());
            return Ok(Admission::UnknownToken);
        };
        (hb, settings::get_or(&conn, "rl_ping_per_s", 1).await)
    };
    let bucket = match p.kind {
        PingKind::Ok => format!("ping:{}", hb.id),
        PingKind::Fail => format!("pingfail:{}", hb.id),
    };
    let limit = u32::try_from(limit.max(1)).unwrap_or(u32::MAX);
    if let Verdict::Deny(retry) = limiter.hit(&bucket, limit, 1, now()) {
        return Ok(Admission::Limited(retry));
    }
    Ok(Admission::Admitted(Admitted {
        token: hb.token,
        kind: p.kind,
        source_ip: p.source_ip.map(|ip| ip.to_string()),
    }))
}

/// AC-13.01, AC-14.01, AC-17.03, AC-18.01. An OK ping pushes the deadline out and closes an
/// open incident; a fail ping opens one at once and leaves the deadline.
/// Returns false when no heartbeat has the token any more.
pub async fn record(
    db: &Db,
    base_url: &str,
    a: Admitted,
    cause: Option<String>,
) -> anyhow::Result<bool> {
    let Admitted {
        token,
        kind,
        source_ip,
    } = a;
    let t = now();
    let signal = match kind {
        PingKind::Ok => Signal::Ping,
        PingKind::Fail => Signal::FailPing {
            cause: cause
                .clone()
                .unwrap_or_else(|| "The job reported a failure".into()),
        },
    };
    db.write(async |c| {
        // Read again inside the write: the body may have taken seconds, so the heartbeat may have
        // been renamed, re-ranked or deleted since `admit`. Deleted is a 404, not a 500.
        let Some(hb) = store::find_by_token(c, &token).await? else {
            return Ok(false);
        };
        store::insert_ping(c, hb.id, kind, source_ip.as_deref(), cause.as_deref(), t).await?;
        if let Some(ev) = incidents::observe(c, &hb.source(), &signal, t).await? {
            notify::send(c, &ev, base_url).await?;
        }
        Ok(true)
    })
    .await
}

/// Heartbeats past their deadline, each reported once: its deadline is cleared until the next ping.
/// `read` answers the cheap "anything missed?" question before the writer is taken.
pub async fn sweep_missed(
    db: &Db,
    read: &turso::Connection,
    base_url: &str,
    at: i64,
) -> anyhow::Result<usize> {
    if !store::any_missed(read, at).await? {
        return Ok(0);
    }
    // One write per batch: after an outage every heartbeat can be late at once.
    let mut total = 0;
    loop {
        let n = db
            .write(async |c| {
                let missed = store::take_missed(c, at, caps::MISSED_BATCH).await?;
                for hb in &missed {
                    // Q18: in a maintenance window the miss waits. Looking again each minute
                    // and at the window's end opens the incident at once if the job stays
                    // silent, and soon after an End now.
                    if let Some(end) =
                        maintenance::covers(c, SourceKind::Heartbeat, hb.id, at).await?
                    {
                        store::hold(c, hb.id, end.min(at + 60)).await?;
                        continue;
                    }
                    if let Some(ev) =
                        incidents::observe(c, &hb.source(), &Signal::HeartbeatMissed, at).await?
                    {
                        notify::send(c, &ev, base_url).await?;
                    }
                }
                Ok(missed.len())
            })
            .await?;
        total += n;
        if (n as i64) < caps::MISSED_BATCH {
            return Ok(total);
        }
    }
}
