//! Environment configuration. Spec: docs/technical-specs/11-environment-configuration.md.

use std::{net::SocketAddr, path::PathBuf};

use anyhow::{Context, bail};

/// Where the database lives when MONITA_DB_PATH is unset (spec 11).
pub const DEFAULT_DB_PATH: &str = "./data/monita.db";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Env {
    Dev,
    Test,
    Prod,
}

/// `Debug` is written by hand below: it prints `(set)` for the SMTP URL (it holds the mail
/// password) and the reset token, so a `{:?}` in a log never leaks either.
#[derive(Clone)]
pub struct Config {
    pub env: Env,
    pub bind: SocketAddr,
    pub base_url: String,
    pub db_path: PathBuf,
    pub backup_dir: Option<PathBuf>,
    pub backup_keep: usize,
    pub smtp_url: Option<String>,
    pub mail_from: String,
    pub ping_url: Option<String>,
    pub trust_proxy: bool,
    pub digest_utc_hour: u32,
    pub reset_token: Option<String>,
    pub log_filter: String,
    pub log_json: bool,
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let set = |v: &Option<String>| v.as_ref().map(|_| "(set)");
        f.debug_struct("Config")
            .field("env", &self.env)
            .field("bind", &self.bind)
            .field("base_url", &self.base_url)
            .field("db_path", &self.db_path)
            .field("backup_dir", &self.backup_dir)
            .field("backup_keep", &self.backup_keep)
            .field("smtp_url", &set(&self.smtp_url))
            .field("mail_from", &self.mail_from)
            .field("ping_url", &self.ping_url)
            .field("trust_proxy", &self.trust_proxy)
            .field("digest_utc_hour", &self.digest_utc_hour)
            .field("reset_token", &set(&self.reset_token))
            .field("log_filter", &self.log_filter)
            .field("log_json", &self.log_json)
            .finish()
    }
}

/// A two-valued variable: unset means `no`; anything but `yes` or `no` stops the start, so a
/// typo such as `TRUE` cannot silently turn a feature off.
fn flag(value: Option<String>, key: &str, yes: &str, no: &str) -> anyhow::Result<bool> {
    match value.as_deref() {
        None => Ok(false),
        Some(v) if v == yes => Ok(true),
        Some(v) if v == no => Ok(false),
        Some(v) => bail!("{key} must be {yes} or {no}, got {v:?}"),
    }
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Self::from_lookup(|k| std::env::var(k).ok())
    }

    /// Reads every variable through `get`, so tests pass a map instead of the process env.
    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> anyhow::Result<Self> {
        // Trimmed once for every variable: an env file line can carry a trailing space or CR.
        let var = |k: &str| {
            get(k)
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        };
        // No default: a prod host that loses this line must not come up in dev with reset-state.
        let env = match var("MONITA_ENV").as_deref() {
            None => bail!("MONITA_ENV is required: dev, test or prod"),
            Some("dev") => Env::Dev,
            Some("test") => Env::Test,
            Some("prod") => Env::Prod,
            Some(other) => bail!("MONITA_ENV must be dev, test or prod, got {other:?}"),
        };
        let bind = var("MONITA_BIND")
            .unwrap_or_else(|| "127.0.0.1:7466".into())
            .parse()
            .context("MONITA_BIND must be host:port")?;
        let raw_base = var("MONITA_BASE_URL")
            .unwrap_or_else(|| format!("http://localhost:{}", SocketAddr::port(&bind)));
        // Stored as the origin a browser sends in `Origin` (lower-case host, no path), because
        // the CSRF check compares the two byte for byte.
        let parsed = reqwest::Url::parse(&raw_base)
            .ok()
            .filter(|u| matches!(u.scheme(), "http" | "https") && u.host_str().is_some())
            .filter(|u| u.path() == "/" && u.query().is_none() && u.fragment().is_none())
            .context("MONITA_BASE_URL must be an origin such as https://monita.company.co.id")?;
        let base_url = parsed.origin().ascii_serialization();
        // A relay may refuse a sender at `localhost`, so the default names Monita's own host.
        let default_from = format!(
            "Monita <monita@{}>",
            parsed.host_str().unwrap_or("localhost")
        );
        if env == Env::Prod && !base_url.starts_with("https://") {
            bail!("MONITA_BASE_URL must start with https:// when MONITA_ENV=prod");
        }
        let parse_num = |k: &str, default: u64| -> anyhow::Result<u64> {
            var(k).map_or(Ok(default), |v| {
                v.parse().with_context(|| format!("{k} must be a number"))
            })
        };
        let digest_utc_hour = u32::try_from(parse_num("MONITA_DIGEST_UTC_HOUR", 1)?)
            .ok()
            .filter(|h| *h <= 23)
            .context("MONITA_DIGEST_UTC_HOUR must be 0 to 23")?;
        let reset_token = match env {
            Env::Prod => None,
            _ => var("MONITA_RESET_TOKEN"),
        };
        // Checked here so `serve`, `migrate` and `setup` refuse the same values, each naming its
        // variable. The SMTP error is not passed on: it can quote the URL, password included.
        let smtp_url = var("MONITA_SMTP_URL");
        if let Some(u) = &smtp_url {
            lettre::AsyncSmtpTransport::<lettre::Tokio1Executor>::from_url(u).map_err(|_| {
                anyhow::anyhow!("MONITA_SMTP_URL must be a mail server URL such as smtps://user:password@host:465")
            })?;
        }
        let mail_from = var("MONITA_MAIL_FROM").unwrap_or(default_from);
        mail_from.parse::<lettre::message::Mailbox>().map_err(|_| {
            anyhow::anyhow!(
                "MONITA_MAIL_FROM must be an address such as Monita <monita@company.co.id>"
            )
        })?;
        let ping_url = var("MONITA_PING_URL");
        if ping_url
            .as_deref()
            .is_some_and(|u| !super::valid_http_url(u))
        {
            bail!("MONITA_PING_URL must be an http:// or https:// URL");
        }
        let log_filter = var("MONITA_LOG").unwrap_or_else(|| {
            if env == Env::Dev {
                "info,monita=debug"
            } else {
                "info"
            }
            .into()
        });
        tracing_subscriber::EnvFilter::try_new(&log_filter)
            .context("MONITA_LOG must be a log filter such as info or info,monita=debug")?;

        Ok(Self {
            env,
            bind,
            base_url,
            db_path: var("MONITA_DB_PATH")
                .unwrap_or_else(|| DEFAULT_DB_PATH.into())
                .into(),
            backup_dir: var("MONITA_BACKUP_DIR").map(PathBuf::from),
            backup_keep: parse_num("MONITA_BACKUP_KEEP", 7)? as usize,
            smtp_url,
            mail_from,
            ping_url,
            trust_proxy: flag(
                var("MONITA_TRUST_PROXY"),
                "MONITA_TRUST_PROXY",
                "true",
                "false",
            )?,
            digest_utc_hour,
            reset_token,
            log_filter,
            // Unset: json in prod for log shippers, pretty elsewhere (spec 11).
            log_json: match var("MONITA_LOG_FORMAT") {
                None => env == Env::Prod,
                set => flag(set, "MONITA_LOG_FORMAT", "json", "pretty")?,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn cfg(pairs: &[(&str, &str)]) -> anyhow::Result<Config> {
        let m: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Config::from_lookup(|k| m.get(k).cloned())
    }

    #[test]
    fn debug_never_prints_the_mail_password_or_the_reset_token() {
        let c = cfg(&[
            ("MONITA_ENV", "dev"),
            (
                "MONITA_SMTP_URL",
                "smtps://user:hunter2@mail.example.com:465",
            ),
            ("MONITA_RESET_TOKEN", "reset-secret"),
        ])
        .unwrap();
        let text = format!("{c:?}");
        assert!(
            !text.contains("hunter2") && !text.contains("reset-secret"),
            "{text}"
        );
        assert!(text.contains(r#"smtp_url: Some("(set)")"#), "{text}");
        let mail = crate::platform::settings::MailSettings {
            host: "mail.example.com".into(),
            port: 465,
            security: "tls".into(),
            user: Some("user".into()),
            password: Some("hunter2".into()),
            from: "Monita <m@example.com>".into(),
            skip_verify: false,
        };
        assert!(!format!("{mail:?}").contains("hunter2"));
    }

    #[test]
    fn refuses_to_start_without_an_env() {
        assert!(cfg(&[]).is_err());
        let c = cfg(&[("MONITA_ENV", "dev")]).unwrap();
        assert_eq!(c.base_url, "http://localhost:7466");
    }

    #[test]
    fn the_sender_defaults_to_monitas_own_host() {
        let prod = [
            ("MONITA_ENV", "prod"),
            ("MONITA_BASE_URL", "https://Monita.Example.com"),
        ];
        assert_eq!(
            cfg(&prod).unwrap().mail_from,
            "Monita <monita@monita.example.com>"
        );
        let set = cfg(&[prod[0], prod[1], ("MONITA_MAIL_FROM", "ops@example.com")]).unwrap();
        assert_eq!(set.mail_from, "ops@example.com");
    }

    #[test]
    fn mail_logging_and_peer_values_are_refused_at_start_by_name() {
        let dev = ("MONITA_ENV", "dev");
        for (key, bad) in [
            ("MONITA_SMTP_URL", "not a url"),
            ("MONITA_MAIL_FROM", "monita at company"),
            ("MONITA_LOG", "info,monita=loud"),
            ("MONITA_PING_URL", "ftp://peer.example/ping/x"),
        ] {
            let err = cfg(&[dev, (key, bad)]).unwrap_err();
            assert!(format!("{err:#}").starts_with(key), "{key}: {err:#}");
        }
        let secret = cfg(&[dev, ("MONITA_SMTP_URL", "smtps://u:hunter2@ host:465")]).unwrap_err();
        assert!(
            !format!("{secret:#}").contains("hunter2"),
            "the password stays out of the error"
        );
        assert!(cfg(&[dev, ("MONITA_SMTP_URL", "smtps://u:p@mail.example.com:465")]).is_ok());
    }

    #[test]
    fn a_digest_hour_past_23_is_refused_even_when_it_wraps_a_u32() {
        for hour in ["24", "4294967297"] {
            assert!(cfg(&[("MONITA_ENV", "dev"), ("MONITA_DIGEST_UTC_HOUR", hour)]).is_err());
        }
    }

    #[test]
    fn two_valued_settings_refuse_anything_else() {
        let dev = |k, v| cfg(&[("MONITA_ENV", "dev"), (k, v)]);
        assert!(dev("MONITA_TRUST_PROXY", "true").unwrap().trust_proxy);
        assert!(!dev("MONITA_TRUST_PROXY", "false").unwrap().trust_proxy);
        assert!(dev("MONITA_TRUST_PROXY", "1").is_err());
        assert!(dev("MONITA_LOG_FORMAT", "json").unwrap().log_json);
        assert!(dev("MONITA_LOG_FORMAT", "JSON").is_err());
        assert!(
            !dev("MONITA_LOG_FORMAT", "pretty").unwrap().log_json,
            "pretty when set"
        );
        assert!(
            dev("MONITA_TRUST_PROXY", "true \r").unwrap().trust_proxy,
            "env-file whitespace"
        );
        let path = dev("MONITA_DB_PATH", "/var/lib/monita/monita.db\r")
            .unwrap()
            .db_path;
        assert_eq!(
            path.to_str(),
            Some("/var/lib/monita/monita.db"),
            "every variable is trimmed"
        );
    }

    #[test]
    fn prod_logs_json_unless_told_otherwise() {
        let prod = cfg(&[("MONITA_ENV", "prod"), ("MONITA_BASE_URL", "https://x.co")]).unwrap();
        assert!(prod.log_json);
        assert!(!cfg(&[("MONITA_ENV", "dev")]).unwrap().log_json);
    }

    #[test]
    fn the_base_url_is_kept_as_a_browser_origin() {
        let c = cfg(&[
            ("MONITA_ENV", "dev"),
            ("MONITA_BASE_URL", "https://Monita.Example.co/"),
        ])
        .unwrap();
        assert_eq!(c.base_url, "https://monita.example.co");
        assert!(
            cfg(&[
                ("MONITA_ENV", "dev"),
                ("MONITA_BASE_URL", "https://x.co/monita")
            ])
            .is_err()
        );
        assert_eq!(
            cfg(&[("MONITA_ENV", "dev")]).unwrap().log_filter,
            "info,monita=debug"
        );
    }

    #[test]
    fn prod_requires_https() {
        assert!(cfg(&[("MONITA_ENV", "prod"), ("MONITA_BASE_URL", "http://x")]).is_err());
        assert!(cfg(&[("MONITA_ENV", "prod"), ("MONITA_BASE_URL", "https://x")]).is_ok());
    }

    #[test]
    fn prod_never_keeps_a_reset_token() {
        let c = cfg(&[
            ("MONITA_ENV", "prod"),
            ("MONITA_BASE_URL", "https://x"),
            ("MONITA_RESET_TOKEN", "t"),
        ])
        .unwrap();
        assert!(c.reset_token.is_none());
    }
}
