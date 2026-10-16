//! checks module: monitors, heartbeats, pings, results, the scheduler loop.
//! Spec: docs/technical-specs/05-module-definitions.md section 5.3 and 13-check-scheduler.md.

pub mod heartbeat;
pub mod heartbeat_routes;
mod probe;
pub mod routes;
pub mod scheduler;
mod store;

use std::collections::BTreeMap;

use crate::platform::{db::Db, now, token};

pub use probe::{Clients, Target, probe, status_matches};
pub use store::{CheckRow, Monitor};

pub const INTERVALS_S: [i64; 4] = [30, 60, 180, 300];

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
        if self.name.trim().is_empty() || self.name.len() > 80 {
            e.insert("name", "Enter a name of up to 80 characters.");
        }
        let url = self.url.trim();
        if !(url.starts_with("http://") || url.starts_with("https://"))
            || url.len() > 2000
            || url.contains(char::is_whitespace)
        {
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
        if self.keyword.as_ref().is_some_and(|k| k.len() > 200) {
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

/// AC-09.01, AC-11.01: the first check lands at a random point inside the interval.
pub async fn create_monitor(db: &Db, project_id: i64, input: &MonitorInput) -> anyhow::Result<i64> {
    let next = now() + token::random_below(input.interval_s as u64) as i64;
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
    let next = now() + token::random_below(input.interval_s as u64) as i64;
    db.write(async |c| store::update_monitor(c, project_id, id, input, next).await)
        .await
}

pub async fn delete_monitor(db: &Db, project_id: i64, id: i64) -> anyhow::Result<()> {
    db.write(async |c| store::delete_monitor(c, project_id, id).await)
        .await
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
    before: Option<i64>,
    limit: i64,
) -> anyhow::Result<Vec<CheckRow>> {
    let since = now() - 7 * 86_400;
    store::recent_checks(
        &db.read().await?,
        monitor_id,
        since,
        before.unwrap_or(i64::MAX),
        limit,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

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
