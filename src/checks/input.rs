//! The monitor form: what a tech lead may save, checked before anything is written.

use std::collections::BTreeMap;

use super::{INTERVALS_S, probe};

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
}

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
            ..Self::default()
        }
    }

    pub fn validate(&self) -> FieldErrors {
        let mut e = FieldErrors::new();
        if self.name.trim().is_empty() || self.name.chars().count() > 80 {
            e.insert("name", "Enter a name of up to 80 characters.");
        }
        let url = self.url.trim();
        if !crate::platform::valid_http_url(url) {
            e.insert("url", "The URL must start with http:// or https://.");
        }
        if !INTERVALS_S.contains(&self.interval_s) {
            e.insert("interval_s", "Pick one of the listed intervals.");
        }
        if !(1_000..=30_000).contains(&self.timeout_ms) {
            e.insert("timeout_ms", "Use 1 to 30 seconds.");
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
        if !matches!(self.severity.as_str(), "critical" | "warning") {
            e.insert("severity", "Pick critical or warning.");
        }
        let fails = crate::incidents::engine::FAILS_RANGE;
        if !(i64::from(*fails.start())..=i64::from(*fails.end())).contains(&self.fails_to_open) {
            e.insert("fails_to_open", "Enter a number from 1 to 99.");
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
                    "Only POST and PUT send a body. Pick one, or clear the body.",
                );
            }
            Some(b) if b.len() > 65_536 => {
                e.insert("body", "The body can be at most 64 KB.");
            }
            _ => {}
        }
        self.auth_errors(&mut e);
        e
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
                "Enter one header name, such as X-Api-Key. Host and Content-Length are not allowed.",
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
                "Remove the header that Authentication sends. Its value goes in a secret.",
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

    #[test]
    fn a_failure_count_runs_from_1_to_99() {
        let with = |n| MonitorInput {
            name: "API".into(),
            url: "https://x.co".into(),
            fails_to_open: n,
            ..MonitorInput::defaults()
        };
        assert!(errors(&with(1)).is_empty());
        assert!(errors(&with(37)).is_empty());
        assert_eq!(errors(&with(0)), ["fails_to_open"]);
        assert_eq!(errors(&with(100)), ["fails_to_open"]);
    }

    fn errors(m: &MonitorInput) -> Vec<&'static str> {
        m.validate().keys().copied().collect()
    }

    #[test]
    fn each_sign_in_kind_needs_a_secret_and_its_own_field() {
        let base = MonitorInput {
            name: "API".into(),
            url: "https://x.co".into(),
            ..MonitorInput::defaults()
        };
        let with = |kind: &str| MonitorInput {
            auth_kind: kind.into(),
            ..base.clone()
        };
        assert!(errors(&with("")).is_empty(), "empty reads as none");
        assert_eq!(errors(&with("oauth")), ["auth_kind"]);
        assert_eq!(errors(&with("bearer")), ["auth_secret_id"]);
        let secret = |kind: &str| MonitorInput {
            auth_secret_id: Some(1),
            ..with(kind)
        };
        assert!(errors(&secret("bearer")).is_empty());
        assert_eq!(errors(&secret("basic")), ["auth_user"]);
        let colon = MonitorInput {
            auth_user: Some("a:b".into()),
            ..secret("basic")
        };
        assert_eq!(errors(&colon), ["auth_user"]);
        assert_eq!(errors(&secret("header")), ["auth_header"]);
        let host = MonitorInput {
            auth_header: Some("Host".into()),
            ..secret("header")
        };
        assert_eq!(errors(&host), ["auth_header"]);
    }

    #[test]
    fn a_plain_header_may_not_repeat_the_one_authentication_sends() {
        let m = MonitorInput {
            name: "API".into(),
            url: "https://x.co".into(),
            auth_kind: "header".into(),
            auth_secret_id: Some(1),
            auth_header: Some("X-Api-Key".into()),
            headers: "x-api-key: plain".into(),
            ..MonitorInput::defaults()
        };
        assert_eq!(errors(&m), ["headers"]);
        let bearer = MonitorInput {
            auth_kind: "bearer".into(),
            headers: "Authorization: Bearer plain".into(),
            ..m.clone()
        };
        assert_eq!(errors(&bearer), ["headers"]);
        let none = MonitorInput {
            auth_kind: "none".into(),
            ..bearer
        };
        assert!(
            errors(&none).is_empty(),
            "no sign-in: a plain header is fine"
        );
    }
}
