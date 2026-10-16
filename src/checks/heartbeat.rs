//! Heartbeats: a job pings Monita; a missing ping is the failure. Spec 13 section 13.1.

use super::{FieldErrors, store::heartbeats as store};
use crate::platform::{db::Db, now, token};
use crate::{
    incidents::{self, Signal, Source, SourceKind},
    notify,
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
    pub last_ping_at: Option<i64>,
    pub deadline_at: Option<i64>,
    pub created_at: i64,
}

impl Heartbeat {
    /// The incident state wins; with no open incident, a heartbeat that never pinged is waiting.
    pub fn state(&self, incident: Option<incidents::ItemState>) -> (&'static str, &'static str) {
        match incident {
            Some(s) => super::state_label(s),
            None if self.last_ping_at.is_none() => ("unknown", "Waiting"),
            None => ("up", "Up"),
        }
    }

    pub fn source(&self) -> Source {
        Source {
            kind: SourceKind::Heartbeat,
            id: self.id,
            project_id: self.project_id,
            name: self.name.clone(),
            severity: self.severity.clone(),
        }
    }

    pub fn input(&self) -> HeartbeatInput {
        HeartbeatInput {
            name: self.name.clone(),
            period_s: self.period_s,
            grace_s: self.grace_s,
            severity: self.severity.clone(),
            is_public: self.is_public,
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
}

impl Default for HeartbeatInput {
    fn default() -> Self {
        Self {
            name: String::new(),
            period_s: 86_400,
            grace_s: 1_800,
            severity: "warning".into(),
            is_public: false,
        }
    }
}

impl HeartbeatInput {
    pub fn validate(&self) -> FieldErrors {
        let mut e = FieldErrors::new();
        if self.name.trim().is_empty() || self.name.len() > 80 {
            e.insert("name", "Enter a name of up to 80 characters.");
        }
        if !PERIODS_S.iter().any(|(v, _)| *v == self.period_s) {
            e.insert("period_s", "Pick one of the listed periods.");
        }
        if !GRACES_S.iter().any(|(v, _)| *v == self.grace_s) {
            e.insert("grace_s", "Pick one of the listed grace times.");
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

pub struct PingRow {
    pub received_at: i64,
    pub kind: String,
    pub source_ip: Option<String>,
}

/// AC-12.01: a heartbeat that never pings still misses its first deadline.
pub async fn create(db: &Db, project_id: i64, input: &HeartbeatInput) -> anyhow::Result<i64> {
    let tok = token::random_token();
    let deadline = now() + input.period_s + input.grace_s;
    db.write(async |c| store::insert(c, project_id, input, &tok, deadline).await)
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

/// Also closes the heartbeat's open incident.
pub async fn delete(db: &Db, project_id: i64, id: i64) -> anyhow::Result<()> {
    db.write(async |c| {
        if store::find(c, project_id, id).await?.is_some() {
            incidents::source_deleted(c, SourceKind::Heartbeat, id, now()).await?;
            store::delete(c, project_id, id).await?;
        }
        Ok(())
    })
    .await
}

pub async fn list(db: &Db, project_id: i64) -> anyhow::Result<Vec<Heartbeat>> {
    store::list(&db.read().await?, project_id).await
}

pub async fn find(db: &Db, project_id: i64, id: i64) -> anyhow::Result<Option<Heartbeat>> {
    store::find(&db.read().await?, project_id, id).await
}

pub async fn find_by_token(db: &Db, tok: &str) -> anyhow::Result<Option<Heartbeat>> {
    store::find_by_token(&db.read().await?, tok).await
}

pub async fn recent_pings(db: &Db, id: i64, limit: i64) -> anyhow::Result<Vec<PingRow>> {
    store::recent_pings(&db.read().await?, id, now() - 7 * 86_400, limit).await
}

/// AC-13.01, AC-14.01, AC-17.03, AC-18.01. An OK ping pushes the deadline out and closes an
/// open incident; a fail ping opens one at once and leaves the deadline.
pub async fn record_ping(
    db: &Db,
    base_url: &str,
    hb: &Heartbeat,
    kind: PingKind,
    source_ip: Option<String>,
    cause: Option<String>,
) -> anyhow::Result<()> {
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
        store::insert_ping(c, hb.id, kind, source_ip.as_deref(), cause.as_deref(), t).await?;
        if let Some(ev) = incidents::observe(c, &hb.source(), &signal, t).await? {
            notify::send(c, &ev, base_url).await?;
        }
        Ok(())
    })
    .await
}

/// Heartbeats past their deadline, each reported once: its deadline is cleared until the next ping.
pub async fn sweep_missed(db: &Db, base_url: &str, at: i64) -> anyhow::Result<usize> {
    if !store::any_missed(&db.read().await?, at).await? {
        return Ok(0);
    }
    db.write(async |c| {
        let missed = store::take_missed(c, at).await?;
        for hb in &missed {
            if let Some(ev) =
                incidents::observe(c, &hb.source(), &Signal::HeartbeatMissed, at).await?
            {
                notify::send(c, &ev, base_url).await?;
            }
        }
        Ok(missed.len())
    })
    .await
}
