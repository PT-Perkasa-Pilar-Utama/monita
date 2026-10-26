//! The monitor form: what a tech lead may save, checked before anything is written.

use std::collections::BTreeMap;

use super::{INTERVALS_S, probe};

/// A validated monitor form. Field errors are keyed by form field name.
#[derive(Clone, Debug, Default)]
pub struct MonitorInput {
    pub name: String,
    /// `http`, `tcp`, `dns` (v1.1 Q16) or `ping` (Q24); empty reads as http.
    pub kind: String,
    /// The URL for `http`, `host:port` for `tcp`, the host name for `dns` and `ping`.
    pub url: String,
    /// `dns` only: the check passes only when one A or AAAA answer equals this address.
    pub expected_address: Option<String>,
    pub interval_s: i64,
    pub timeout_ms: i64,
    pub expected_status: String,
    pub keyword: Option<String>,
    pub skip_tls_verify: bool,
    pub severity: String,
    pub is_public: bool,
    /// A failure alerts the severity's recipients.
    pub alert: bool,
    /// A failure opens an incident on its own. Off: only an alert, and you report one by hand.
    pub auto_incident: bool,
    /// Failed checks in a row that count as down: 1 to 99.
    pub fails_to_open: i64,
    /// GET, HEAD, POST or PUT; empty reads as GET.
    pub method: String,
    /// `Name: value` lines, as entered.
    pub headers: String,
    /// Sent with POST and PUT only.
    pub body: Option<String>,
    /// `none`, `bearer`, `basic` or `header`; empty reads as none.
    pub auth_kind: String,
    /// The project secret the check sends. Its project is checked when the form is saved.
    pub auth_secret_id: Option<i64>,
    /// The user name for `basic`.
    pub auth_user: Option<String>,
    /// The header name for `header`, such as `X-Api-Key`.
    pub auth_header: Option<String>,
    /// HTTPS only: warn this many days before the certificate expires; None is off.
    pub cert_warn_days: Option<i64>,
    /// How many places must fail for a check to count as failed: 1 to 1 + the locations.
    pub quorum: i64,
    /// The locations the monitor also checks from (technical-specs 14). Their existence is
    /// checked when the form is saved.
    pub location_ids: Vec<i64>,
}

/// The certificate warnings a monitor may choose, in days (v1.1 Q25).
pub const CERT_WARN_DAYS: [i64; 3] = [7, 14, 30];

/// How a check signs in.
pub const AUTH_KINDS: [&str; 4] = ["none", "bearer", "basic", "header"];

pub type FieldErrors = BTreeMap<&'static str, &'static str>;

impl MonitorInput {
    /// The defaults of AC-09.02.
    pub fn defaults() -> Self {
        Self {
            interval_s: 60,
            timeout_ms: 10_000,
            expected_status: "2xx".into(),
            severity: "warning".into(),
            alert: true,
            auto_incident: true,
            fails_to_open: 2,
            quorum: 1,
            ..Self::default()
        }
    }

    pub fn validate(&self) -> FieldErrors {
        let mut e = FieldErrors::new();
        if self.name.trim().is_empty() || self.name.chars().count() > 80 {
            e.insert("name", "Enter a name of up to 80 characters.");
        }
        if !INTERVALS_S.contains(&self.interval_s) {
            e.insert("interval_s", "Pick one of the listed intervals.");
        }
        if !(1_000..=30_000).contains(&self.timeout_ms) {
            e.insert("timeout_ms", "Use 1 to 30 seconds.");
        }
        if !matches!(self.severity.as_str(), "critical" | "warning") {
            e.insert("severity", "Pick critical or warning.");
        }
        let fails = crate::incidents::engine::FAILS_RANGE;
        if !(i64::from(*fails.start())..=i64::from(*fails.end())).contains(&self.fails_to_open) {
            e.insert("fails_to_open", "Enter a number from 1 to 99.");
        }
        let places = 1 + self.location_ids.len() as i64;
        if !(1..=places).contains(&self.quorum) {
            e.insert(
                "quorum",
                if places == 1 {
                    "With this server alone, 1 place decides."
                } else {
                    "Enter a number from 1 to the number of places."
                },
            );
        }
        let url = self.url.trim();
        match self.kind() {
            "http" => self.http_errors(&mut e),
            "tcp" => {
                if !valid_tcp_target(url) {
                    e.insert(
                        "url",
                        "Enter a host and a port, such as db.internal:5432 or [2001:db8::1]:443.",
                    );
                }
            }
            "dns" => {
                if !valid_host(url) {
                    e.insert("url", "Enter a host name, such as api.example.com.");
                }
                let expected = self.expected_address.as_deref().map(str::trim);
                if expected.is_some_and(|a| !a.is_empty() && a.parse::<std::net::IpAddr>().is_err())
                {
                    e.insert(
                        "expected_address",
                        "Enter an IPv4 or IPv6 address, such as 203.0.113.10, or leave it empty.",
                    );
                }
            }
            "ping" => {
                if !valid_host(url) {
                    e.insert(
                        "url",
                        "Enter a host name or an address, such as db.internal or 10.0.0.5.",
                    );
                }
            }
            _ => {
                e.insert("kind", "Pick HTTP, TCP, DNS or ping.");
            }
        }
        e
    }

    /// The fields only an HTTP check reads.
    fn http_errors(&self, e: &mut FieldErrors) {
        let url = self.url.trim();
        if !crate::platform::valid_http_url(url) {
            e.insert("url", "The URL must start with http:// or https://.");
        } else if crate::platform::has_userinfo(url) {
            e.insert(
                "url",
                "Put the user name and password under Sign in with, not in the URL.",
            );
        }
        if !valid_expected_status(&self.expected_status) {
            e.insert(
                "expected_status",
                "Use a class such as 2xx, or a list of codes such as 200,204.",
            );
        }
        if self
            .keyword
            .as_ref()
            .is_some_and(|k| k.chars().count() > 200)
        {
            e.insert("keyword", "Use at most 200 characters.");
        }
        if !probe::METHODS.contains(&self.method()) {
            e.insert("method", "Pick GET, HEAD, POST or PUT.");
        }
        if let Err(m) = probe::parse_headers(&self.headers) {
            e.insert("headers", m);
        }
        match &self.body {
            Some(b) if !matches!(self.method(), "POST" | "PUT") && !b.is_empty() => {
                e.insert(
                    "body",
                    "Only POST and PUT send a body, so pick one of them or clear the body.",
                );
            }
            Some(b) if b.len() > 65_536 => {
                e.insert("body", "The body can be at most 64 KB.");
            }
            _ => {}
        }
        self.auth_errors(e);
        if let Some(days) = self.cert_warn_days {
            if !CERT_WARN_DAYS.contains(&days) {
                e.insert("cert_warn_days", "Pick 7, 14 or 30 days, or off.");
            } else if !url.to_ascii_lowercase().starts_with("https://") {
                e.insert(
                    "cert_warn_days",
                    "Only an https:// URL has a certificate to watch, so turn the warning off.",
                );
            }
        }
    }

    /// The form as it is stored: a TCP or DNS check keeps none of the HTTP fields, and only a DNS
    /// check keeps an expected address.
    pub fn for_kind(&self) -> MonitorInput {
        let mut m = self.clone();
        m.kind = self.kind().to_string();
        m.url = self.url.trim().to_string();
        if m.kind != "dns" {
            m.expected_address = None;
        }
        m.expected_address = m
            .expected_address
            .map(|a| a.trim().to_string())
            .filter(|a| !a.is_empty());
        if m.kind != "http" {
            let keep = MonitorInput::defaults();
            m.expected_status = keep.expected_status;
            m.keyword = None;
            m.skip_tls_verify = false;
            m.method = String::new();
            m.headers = String::new();
            m.body = None;
            m.auth_kind = String::new();
            m.auth_secret_id = None;
            m.auth_user = None;
            m.auth_header = None;
            m.cert_warn_days = None;
        }
        m
    }

    pub fn kind(&self) -> &str {
        if self.kind.is_empty() {
            "http"
        } else {
            &self.kind
        }
    }

    fn auth_errors(&self, e: &mut FieldErrors) {
        let kind = self.auth_kind();
        if !AUTH_KINDS.contains(&kind) {
            e.insert("auth_kind", "Pick how the check signs in.");
            return;
        }
        if kind == "none" {
            return;
        }
        if self.auth_secret_id.is_none() {
            e.insert("auth_secret_id", "Pick a secret.");
        }
        let user = self.auth_user.as_deref().unwrap_or_default();
        if kind == "basic" && (user.is_empty() || user.contains(':') || user.len() > 200) {
            e.insert(
                "auth_user",
                "Enter a user name of up to 200 characters, without a colon.",
            );
        }
        let header = self.auth_header.as_deref().unwrap_or_default();
        if kind == "header" && probe::parse_headers(&format!("{header}: x")).is_err() {
            e.insert(
                "auth_header",
                "Enter one header name such as X-Api-Key, other than Host or Content-Length.",
            );
        }
        let sent = if kind == "header" {
            header
        } else {
            "authorization"
        };
        let twice = probe::parse_headers(&self.headers)
            .unwrap_or_default()
            .iter()
            .any(|(name, _)| name.eq_ignore_ascii_case(sent));
        if twice {
            e.insert(
                "headers",
                "Remove the header that Authentication sends, because its value goes in a secret.",
            );
        }
    }

    pub fn auth_kind(&self) -> &str {
        if self.auth_kind.is_empty() {
            "none"
        } else {
            &self.auth_kind
        }
    }

    pub fn method(&self) -> &str {
        if self.method.is_empty() {
            "GET"
        } else {
            &self.method
        }
    }
}

/// A host name or an IP address, without a port. Names may hold letters, digits, `-`, `_` and
/// dots, as internal names often do.
pub fn valid_host(host: &str) -> bool {
    if host.parse::<std::net::IpAddr>().is_ok() {
        return true;
    }
    (1..=253).contains(&host.len())
        && !host.starts_with(['.', '-'])
        && !host.ends_with('-')
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

/// `host:port`, or `[ipv6]:port`, with a port from 1 to 65535.
pub fn valid_tcp_target(target: &str) -> bool {
    let (host, port) = match target.strip_prefix('[') {
        Some(rest) => match rest.split_once("]:") {
            Some((ip, port)) if ip.parse::<std::net::Ipv6Addr>().is_ok() => (None, port),
            _ => return false,
        },
        None => match target.rsplit_once(':') {
            Some((host, port)) if !host.contains(':') => (Some(host), port),
            _ => return false,
        },
    };
    host.is_none_or(valid_host) && port.parse::<u16>().is_ok_and(|p| p > 0)
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

#[cfg(test)]
mod tests;
