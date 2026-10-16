//! Heartbeats: a job pings Monita; a missing ping is the failure. Spec 13 section 13.1.

use super::{FieldErrors, store::heartbeats as store};
use crate::platform::{
    db::Db,
    now,
    rate_limit::{self, Limiter, Verdict},
    settings, token,
};
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
        if self.name.trim().is_empty() || self.name.chars().count() > 80 {
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

pub async fn list(db: &Db, project_id: i64) -> anyhow::Result<Vec<Heartbeat>> {
    store::list(&db.read().await?, project_id).await
}

pub async fn find(db: &Db, project_id: i64, id: i64) -> anyhow::Result<Option<Heartbeat>> {
    store::find(&db.read().await?, project_id, id).await
}

pub async fn recent_pings(db: &Db, id: i64, limit: i64) -> anyhow::Result<Vec<PingRow>> {
    store::recent_pings(&db.read().await?, id, now() - 7 * 86_400, limit).await
}

/// Proof that `admit` passed. Only `admit` makes one and only `record` takes it, so no caller can
/// store a ping that skipped the token and the limits.
pub struct Admitted {
    /// Ids only: `record` reads the heartbeat again, so nothing here can be stale.
    project_id: i64,
    id: i64,
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

/// The cause a `/fail` ping keeps: its first 1 024 bytes after leading whitespace, cut on a
/// character boundary. A longer body is cut, never refused, so the failure itself is always
/// recorded (api-specs 04 section 4.2). Fed chunk by chunk, so it never holds more than that.
#[derive(Default)]
pub struct Cause {
    kept: Vec<u8>,
    /// Bytes of a character split across chunks while still in the leading whitespace.
    pending: Vec<u8>,
    started: bool,
}

impl Cause {
    /// 1 KB and one more character, so the cut can land on a boundary.
    const KEEP: usize = 1024 + 3;

    pub fn push(&mut self, chunk: &[u8]) {
        let joined;
        let mut chunk = chunk;
        if !self.started {
            // Leading Unicode whitespace is skipped before it can use up the budget.
            joined = [std::mem::take(&mut self.pending).as_slice(), chunk].concat();
            let valid = match std::str::from_utf8(&joined) {
                Ok(s) => s,
                Err(e) => std::str::from_utf8(&joined[..e.valid_up_to()]).unwrap_or_default(),
            };
            let skip = valid.len() - valid.trim_start().len();
            let rest = &joined[skip..];
            let cut_char = matches!(std::str::from_utf8(rest), Err(e) if e.error_len().is_none());
            if skip == valid.len() && cut_char {
                // Only whitespace so far, then a character cut at the chunk's end: wait for it.
                self.pending = rest.to_vec();
                return;
            }
            if rest.is_empty() {
                return;
            }
            self.started = true;
            chunk = rest;
        }
        let room = Self::KEEP.saturating_sub(self.kept.len());
        self.kept.extend_from_slice(&chunk[..chunk.len().min(room)]);
    }

    /// True once the cause has all it keeps; the rest of the body can be dropped unread.
    pub fn is_full(&self) -> bool {
        self.kept.len() >= Self::KEEP
    }

    pub fn finish(self) -> Option<String> {
        let text = String::from_utf8_lossy(&self.kept);
        let text = text.trim();
        let mut end = text.len().min(1024);
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        (end > 0).then(|| text[..end].to_string())
    }
}

/// Unknown-token pings allowed per minute from one IP before 429. A real job never misses;
/// a scanner does, and each miss costs a lookup.
const MISSES_PER_MIN: u32 = 60;

/// The checks before a ping is read or stored: the token, then the limits. AC-13.02: over
/// `rl_ping_per_s` a ping is refused, and the ping already counted this second keeps the
/// heartbeat alive. `/fail` has its own bucket, so a job that pings and then fails in one second
/// is still heard. A caller reads a `/fail` body only after `Admitted`.
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
        project_id: hb.project_id,
        id: hb.id,
        kind: p.kind,
        source_ip: p.source_ip.map(|ip| ip.to_string()),
    }))
}

/// AC-13.01, AC-14.01, AC-17.03, AC-18.01. An OK ping pushes the deadline out and closes an
/// open incident; a fail ping opens one at once and leaves the deadline.
/// Returns false when the heartbeat was deleted after `admit`.
pub async fn record(
    db: &Db,
    base_url: &str,
    a: Admitted,
    cause: Option<String>,
) -> anyhow::Result<bool> {
    let Admitted {
        project_id,
        id,
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
        let Some(hb) = store::find(c, project_id, id).await? else {
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

#[cfg(test)]
mod tests {
    use super::Cause;

    fn cause(chunks: &[&[u8]]) -> Option<String> {
        let mut c = Cause::default();
        for chunk in chunks {
            c.push(chunk);
        }
        c.finish()
    }

    #[test]
    fn a_cause_is_cut_to_1_kb_on_a_character_boundary() {
        assert_eq!(cause(&[b"disk full"]).as_deref(), Some("disk full"));
        let long = "x".repeat(1023) + "\u{e9}";
        assert_eq!(
            cause(&[long.as_bytes()]).unwrap().len(),
            1023,
            "the two-byte character goes whole"
        );
        assert_eq!(cause(&[&[b'y'; 5000]]).unwrap().len(), 1024);
    }

    #[test]
    fn leading_whitespace_does_not_use_up_the_cause() {
        let blank = vec![b'\n'; 1100];
        assert_eq!(cause(&[&blank, b"Error: x"]).as_deref(), Some("Error: x"));
        assert_eq!(cause(&[b"  \n "]), None, "a blank body has no cause");
        let unicode = "\u{a0}\u{2003}\x0bError: y".as_bytes();
        assert_eq!(cause(&[unicode]).as_deref(), Some("Error: y"));
        let wide = "\u{2003}".repeat(600) + "Error: db down";
        let (head, tail) = wide.as_bytes().split_at(1000); // splits one em space across chunks
        assert_eq!(cause(&[head, tail]).as_deref(), Some("Error: db down"));
        assert_eq!(
            cause(&[b"\n\xFFab"]).as_deref(),
            Some("\u{fffd}ab"),
            "an invalid byte is kept, not held as a cut character"
        );
    }
}
