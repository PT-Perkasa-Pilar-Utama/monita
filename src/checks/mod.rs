//! checks module: monitors, heartbeats, pings, results, the scheduler loop.
//! Spec: docs/technical-specs/05-module-definitions.md section 5.3 and 13-check-scheduler.md.

pub mod heartbeat;
pub mod heartbeat_routes;
mod probe;
pub mod retention;
pub mod routes;
pub mod scheduler;
mod store;

use std::collections::BTreeMap;

use crate::platform::{db::Db, now, token};

pub use probe::{Clients, Target, probe, status_matches};
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
        Some(crate::incidents::ItemState::Up) => ("up", "Up"),
        None if !seen => ("unknown", "Waiting"),
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
}

/// A validated monitor form. Field errors are keyed by form field name.
#[derive(Clone, Debug, Default)]
pub struct MonitorInput {
    pub name: String,
    pub url: String,
    pub interval_s: i64,
    pub timeout_ms: i64,
    pub expected_status: String,
    pub keyword: Option<String>,
    pub skip_tls_verify: bool,
    pub severity: String,
    pub is_public: bool,
}

pub type FieldErrors = BTreeMap<&'static str, &'static str>;

impl MonitorInput {
    /// The defaults of AC-09.02.
    pub fn defaults() -> Self {
        Self {
            interval_s: 60,
            timeout_ms: 10_000,
            expected_status: "2xx".into(),
            severity: "warning".into(),
            ..Self::default()
        }
    }

    pub fn validate(&self) -> FieldErrors {
        let mut e = FieldErrors::new();
        if self.name.trim().is_empty() || self.name.chars().count() > 80 {
            e.insert("name", "Enter a name of up to 80 characters.");
        }
        let url = self.url.trim();
        if !crate::platform::valid_http_url(url) || url.contains(char::is_whitespace) {
            e.insert("url", "The URL must start with http:// or https://.");
        }
        if !INTERVALS_S.contains(&self.interval_s) {
            e.insert("interval_s", "Pick one of the listed intervals.");
        }
        if !(1_000..=30_000).contains(&self.timeout_ms) {
            e.insert("timeout_ms", "Use 1 to 30 seconds.");
        }
        if !valid_expected_status(&self.expected_status) {
            e.insert("expected_status", "Use 2xx, or codes such as 200,204.");
        }
        if self
            .keyword
            .as_ref()
            .is_some_and(|k| k.chars().count() > 200)
        {
            e.insert("keyword", "Use at most 200 characters.");
        }
        if !matches!(self.severity.as_str(), "critical" | "warning") {
            e.insert("severity", "Pick critical or warning.");
        }
        e
    }
}

/// `2xx`, `200`, or a comma list such as `200,204,301`.
pub fn valid_expected_status(s: &str) -> bool {
    let s = s.trim();
    !s.is_empty()
        && s.split(',').all(|part| {
            let p = part.trim().as_bytes();
            p.len() == 3
                && (b'1'..=b'5').contains(&p[0])
                && ((p[1] == b'x' && p[2] == b'x')
                    || (p[1].is_ascii_digit() && p[2].is_ascii_digit()))
        })
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

pub async fn public_items(db: &Db, project_id: i64) -> anyhow::Result<Vec<PublicItem>> {
    use crate::incidents::SourceKind;
    Ok(store::public_items(&db.read().await?, project_id)
        .await?
        .into_iter()
        .map(|(hb, id, name, created_at, seen)| PublicItem {
            kind: if hb {
                SourceKind::Heartbeat
            } else {
                SourceKind::Monitor
            },
            id,
            seen,
            name,
            created_at,
        })
        .collect())
}

/// AC-11.01: a random point inside the interval, so checks spread out. Never sooner than 5 s, so
/// the TL sees the new monitor as Waiting before its first result.
fn first_check_offset(interval_s: i64) -> i64 {
    const FLOOR_S: i64 = 5;
    FLOOR_S + token::random_below((interval_s - FLOOR_S).max(1) as u64) as i64
}

/// AC-09.01, AC-11.01: the first check lands at a random point inside the interval.
pub async fn create_monitor(db: &Db, project_id: i64, input: &MonitorInput) -> anyhow::Result<i64> {
    let next = now() + first_check_offset(input.interval_s);
    db.write(async |c| store::insert_monitor(c, project_id, input, next).await)
        .await
}

/// A changed interval gets a fresh random offset, so edits keep checks spread.
pub async fn update_monitor(
    db: &Db,
    project_id: i64,
    id: i64,
    input: &MonitorInput,
) -> anyhow::Result<bool> {
    let next = now() + first_check_offset(input.interval_s);
    db.write(async |c| store::update_monitor(c, project_id, id, input, next).await)
        .await
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
        Ok(())
    })
    .await
}

/// The names of a project's monitors and heartbeats, on the caller's connection, so a writer can
/// read them inside its own transaction.
pub async fn source_names(
    c: &turso::Connection,
    project_id: i64,
) -> anyhow::Result<Vec<(crate::incidents::SourceKind, i64, String)>> {
    use crate::incidents::SourceKind;
    Ok(store::source_names(c, project_id)
        .await?
        .into_iter()
        .map(|(hb, id, name)| {
            let kind = if hb {
                SourceKind::Heartbeat
            } else {
                SourceKind::Monitor
            };
            (kind, id, name)
        })
        .collect())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_limit_counts_characters_not_bytes() {
        let input = MonitorInput {
            name: "é".repeat(80),
            url: "https://x.co".into(),
            ..MonitorInput::defaults()
        };
        assert!(!input.validate().contains_key("name"));
    }

    #[test]
    fn the_first_check_lands_inside_the_interval_but_not_at_once() {
        for _ in 0..200 {
            let o = first_check_offset(30);
            assert!((5..30).contains(&o), "{o}");
        }
    }

    #[test]
    fn expected_status_accepts_classes_and_lists() {
        assert!(valid_expected_status("2xx"));
        assert!(valid_expected_status("200, 204,301"));
        assert!(!valid_expected_status("2x0"));
        assert!(!valid_expected_status("600"));
        assert!(!valid_expected_status("ok"));
        assert!(!valid_expected_status(""));
    }

    #[test]
    fn defaults_pass_validation_once_name_and_url_are_set() {
        let mut m = MonitorInput::defaults();
        assert!(m.validate().contains_key("name"));
        m.name = "API health".into();
        m.url = "https://pay.internal/api/health".into();
        assert!(m.validate().is_empty());
        m.interval_s = 45;
        assert_eq!(
            m.validate().keys().copied().collect::<Vec<_>>(),
            vec!["interval_s"]
        );
    }
}
