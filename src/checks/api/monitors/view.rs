//! A monitor as its rows and pages show it, with what a supervisor or a read key must not see.

use super::*;

/// A monitor as its rows and pages show it.
#[derive(Serialize)]
pub(super) struct MonitorView {
    pub(super) id: i64,
    pub(super) name: String,
    /// `http`, `tcp`, `dns` or `ping`.
    pub(super) kind: String,
    /// The URL, `host:port` for TCP, or the host name for DNS.
    pub(super) url: String,
    /// DNS only: the address one answer must equal.
    pub(super) expected_address: Option<String>,
    /// HTTPS only: days of warning before the certificate expires; null is off.
    pub(super) cert_warn_days: Option<i64>,
    /// The `notAfter` of the certificate the last HTTPS check saw.
    pub(super) cert_expires_at: Option<i64>,
    pub(super) interval_s: i64,
    /// `30 seconds`, `1 minute`, `3 minutes` or `5 minutes`.
    pub(super) every: &'static str,
    pub(super) timeout_s: i64,
    pub(super) expected_status: String,
    pub(super) keyword: Option<String>,
    pub(super) skip_tls_verify: bool,
    pub(super) severity: String,
    pub(super) is_public: bool,
    pub(super) alert: bool,
    pub(super) auto_incident: bool,
    pub(super) fails_to_open: i64,
    /// How many places must fail (technical-specs 14.7).
    pub(super) quorum: i64,
    pub(super) method: String,
    /// `Name: value` lines. Owner and super admin only, like the rest of this view.
    pub(super) headers: String,
    pub(super) body: Option<String>,
    /// `none`, `bearer`, `basic` or `header`. Never the secret's value.
    pub(super) auth_kind: String,
    pub(super) auth_secret_id: Option<i64>,
    /// Set on one monitor's page only.
    pub(super) auth_secret_name: Option<String>,
    pub(super) auth_user: Option<String>,
    pub(super) auth_header: Option<String>,
    pub(super) created_at: i64,
    pub(super) next_check_at: i64,
    /// `up`, `down`, `issue`, `maintenance`, `paused` or `unknown` (waiting for the first check).
    pub(super) state: &'static str,
    /// When a tech lead paused it; null while it runs.
    pub(super) paused_at: Option<i64>,
    pub(super) last: Option<LastCheck>,
}

#[derive(Serialize)]
pub(super) struct LastCheck {
    checked_at: i64,
    ok: bool,
    /// `200 in 182ms`, or the start of the error.
    summary: String,
}

/// What a supervisor sees of a request: header names and whether a body or a sign-in name is
/// sent, never the values, which often carry credentials (technical-specs 07: owner and super admin only).
const HIDDEN: &str = "(hidden)";

fn hide_values(headers: &str) -> String {
    headers
        .lines()
        .map(|l| match l.split_once(':') {
            Some((name, _)) => format!("{name}: {HIDDEN}"),
            None => HIDDEN.to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// A URL's query and fragment, where tokens such as `?apikey=` live, read `(hidden)`.
pub(super) fn hide_query(url: &str) -> String {
    match url.find(['?', '#']) {
        Some(i) => format!("{}{}{HIDDEN}", &url[..i], &url[i..=i]),
        None => url.to_string(),
    }
}

pub(super) fn view(m: Monitor, state: &'static str, read_only: bool) -> MonitorView {
    let headers = m.headers.unwrap_or_default();
    MonitorView {
        id: m.id,
        every: interval_label(m.interval_s),
        interval_s: m.interval_s,
        timeout_s: m.timeout_ms / 1000,
        expected_status: m.expected_status,
        keyword: m.keyword,
        skip_tls_verify: m.skip_tls_verify,
        severity: m.severity,
        is_public: m.is_public,
        alert: m.alert,
        auto_incident: m.auto_incident,
        fails_to_open: m.fails_to_open,
        quorum: m.quorum,
        method: m.method,
        headers: if read_only {
            hide_values(&headers)
        } else {
            headers
        },
        body: m
            .body
            .map(|b| if read_only { HIDDEN.to_string() } else { b }),
        auth_kind: m.auth_kind,
        auth_secret_id: m.auth_secret_id,
        auth_secret_name: None,
        auth_user: m
            .auth_user
            .map(|u| if read_only { HIDDEN.to_string() } else { u }),
        auth_header: m.auth_header,
        created_at: m.created_at,
        next_check_at: m.next_check_at,
        paused_at: m.paused_at,
        state,
        last: m.last.as_ref().map(|c: &CheckRow| LastCheck {
            checked_at: c.checked_at,
            ok: c.ok,
            summary: check_summary(c),
        }),
        name: m.name,
        kind: m.kind,
        url: if read_only { hide_query(&m.url) } else { m.url },
        expected_address: m.expected_address,
        cert_warn_days: m.cert_warn_days,
        cert_expires_at: m.cert_expires_at,
    }
}
