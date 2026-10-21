//! The monitor form as the page sends it, before validation.

use serde::Deserialize;

use crate::checks::MonitorInput;

/// Strings, not numbers: a blank or bad number must reach `validate` and come back as that
/// field's error.
#[derive(Deserialize, Default)]
#[serde(default)]
pub(super) struct MonitorForm {
    name: String,
    /// `http`, `tcp`, `dns` or `ping`; missing means http, as every monitor was before.
    kind: String,
    url: String,
    /// `dns` only.
    expected_address: String,
    interval_s: String,
    timeout_s: String,
    expected_status: String,
    keyword: String,
    skip_tls_verify: bool,
    severity: String,
    is_public: bool,
    /// Missing means on, so a client that never sends it keeps alerts and incidents.
    alert: Option<bool>,
    auto_incident: Option<bool>,
    method: String,
    headers: String,
    body: String,
    /// Missing means 2, the count every monitor used before it could choose.
    fails_to_open: Option<String>,
    auth_kind: String,
    auth_secret_id: Option<i64>,
    auth_user: String,
    auth_header: String,
    /// `7`, `14` or `30`; empty or missing is off.
    cert_warn_days: String,
    /// Locations the monitor also checks from; missing is none (technical-specs 14).
    location_ids: Vec<i64>,
    /// How many places must fail; missing means 1.
    quorum: Option<String>,
}

impl From<MonitorForm> for MonitorInput {
    fn from(f: MonitorForm) -> Self {
        let keyword = f.keyword.trim();
        MonitorInput {
            name: f.name.trim().to_string(),
            kind: f.kind.trim().to_lowercase(),
            url: f.url.trim().to_string(),
            expected_address: Some(f.expected_address.trim().to_string()).filter(|a| !a.is_empty()),
            // 0 fails validation with the field's own message.
            interval_s: f.interval_s.trim().parse().unwrap_or(0),
            timeout_ms: f
                .timeout_s
                .trim()
                .parse::<i64>()
                .unwrap_or(0)
                .saturating_mul(1000),
            expected_status: f.expected_status.trim().to_string(),
            keyword: (!keyword.is_empty()).then(|| keyword.to_string()),
            skip_tls_verify: f.skip_tls_verify,
            severity: f.severity,
            is_public: f.is_public,
            alert: f.alert.unwrap_or(true),
            auto_incident: f.auto_incident.unwrap_or(true),
            fails_to_open: f.fails_to_open.map_or(2, |n| n.trim().parse().unwrap_or(0)),
            quorum: f.quorum.map_or(1, |n| n.trim().parse().unwrap_or(0)),
            location_ids: {
                let mut ids = f.location_ids;
                ids.sort_unstable();
                ids.dedup();
                ids
            },
            method: f.method.trim().to_uppercase(),
            headers: f.headers,
            body: Some(f.body).filter(|b| !b.trim().is_empty()),
            auth_kind: f.auth_kind.trim().to_string(),
            auth_secret_id: f.auth_secret_id,
            auth_user: Some(f.auth_user.trim().to_string()).filter(|u| !u.is_empty()),
            auth_header: Some(f.auth_header.trim().to_string()).filter(|h| !h.is_empty()),
            // A bad number is 0, which fails validation with the field's own message.
            cert_warn_days: Some(f.cert_warn_days.trim())
                .filter(|d| !d.is_empty())
                .map(|d| d.parse().unwrap_or(0)),
        }
    }
}
