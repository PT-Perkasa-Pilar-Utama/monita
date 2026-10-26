//! One check as data: what the scheduler runs on this server, and what a location's probe
//! receives (technical-specs 14.5). It carries the sign-in secret's value when the monitor signs
//! in, so it has no `Debug` and is never logged.

use serde::{Deserialize, Serialize};

use super::probe::{self, Auth, Clients, Target};
use super::{CheckResult, Monitor, SECRET_GONE};
use crate::platform::now;

#[derive(Serialize, Deserialize, Clone)]
pub struct Job {
    pub monitor_id: i64,
    /// `http`, `tcp`, `dns` or `ping`.
    pub kind: String,
    /// The URL, `host:port`, or the host.
    pub target: String,
    pub method: String,
    /// `Name: value` lines.
    pub headers: String,
    pub body: Option<String>,
    pub timeout_ms: i64,
    pub expected_status: String,
    pub keyword: Option<String>,
    pub skip_tls_verify: bool,
    pub expected_address: Option<String>,
    /// How the check signs in, with the secret's value. Null when it does not.
    pub auth: Option<JobAuth>,
    /// The monitor signs in, but its secret was deleted: the check fails with `SECRET_GONE`.
    #[serde(default)]
    pub secret_gone: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct JobAuth {
    /// `bearer`, `basic` or `header`.
    pub kind: String,
    pub user: Option<String>,
    pub header: Option<String>,
    pub secret: String,
}

impl Job {
    /// The monitor's check. `secret` is the value of its sign-in secret, from `read_secret`.
    pub fn new(m: &Monitor, secret: Option<String>) -> Self {
        let signs_in = m.kind == "http" && m.auth_kind != "none";
        Self {
            monitor_id: m.id,
            kind: m.kind.clone(),
            target: m.url.clone(),
            method: m.method.clone(),
            headers: m.headers.clone().unwrap_or_default(),
            body: m.body.clone(),
            timeout_ms: m.timeout_ms,
            expected_status: m.expected_status.clone(),
            keyword: m.keyword.clone(),
            skip_tls_verify: m.skip_tls_verify,
            expected_address: m.expected_address.clone(),
            secret_gone: signs_in && secret.is_none(),
            auth: secret.filter(|_| signs_in).map(|secret| JobAuth {
                kind: m.auth_kind.clone(),
                user: m.auth_user.clone(),
                header: m.auth_header.clone(),
                secret,
            }),
        }
    }

    /// The value of the monitor's sign-in secret, on the caller's connection. None when it does
    /// not sign in or the secret is gone. Only an HTTP check that signs in reads anything.
    pub async fn read_secret(
        c: &crate::platform::sql::Connection,
        m: &Monitor,
    ) -> anyhow::Result<Option<String>> {
        match m.auth_secret_id {
            Some(id) if Self::signs_in(m) => {
                crate::projects::secrets::value_of(c, m.project_id, id).await
            }
            _ => Ok(None),
        }
    }

    /// Whether the check sends a secret, so building its job reads one.
    pub fn signs_in(m: &Monitor) -> bool {
        m.kind == "http" && m.auth_kind != "none"
    }

    /// Runs the check. Never fails: every problem becomes a failed result.
    pub async fn run(&self, clients: &Clients) -> CheckResult {
        match self.kind.as_str() {
            "tcp" => probe::probe_tcp(&self.target, self.timeout_ms).await,
            "ping" => probe::icmp::probe_ping(&self.target, self.timeout_ms).await,
            "dns" => {
                let expected = self.expected_address.as_deref();
                probe::probe_dns(&self.target, expected, self.timeout_ms).await
            }
            _ if self.secret_gone => CheckResult {
                checked_at: now(),
                ok: false,
                status_code: None,
                latency_ms: 0,
                error: Some(SECRET_GONE.into()),
                cert_expires_at: None,
            },
            _ => probe::probe(&self.http_target(), clients).await,
        }
    }

    fn http_target(&self) -> Target<'_> {
        let auth = self.auth.as_ref().and_then(|a| match a.kind.as_str() {
            "bearer" => Some(Auth::Bearer(&a.secret)),
            "basic" => Some(Auth::Basic {
                user: a.user.as_deref().unwrap_or_default(),
                password: &a.secret,
            }),
            "header" => Some(Auth::Header {
                name: a.header.as_deref().unwrap_or_default(),
                value: &a.secret,
            }),
            _ => None,
        });
        Target {
            url: &self.target,
            method: &self.method,
            // Validated on save, so a parse error here means an edited row; send no headers.
            headers: probe::parse_headers(&self.headers).unwrap_or_default(),
            body: self.body.as_deref(),
            timeout_ms: self.timeout_ms,
            expected_status: &self.expected_status,
            keyword: self.keyword.as_deref(),
            skip_tls_verify: self.skip_tls_verify,
            auth,
        }
    }
}
