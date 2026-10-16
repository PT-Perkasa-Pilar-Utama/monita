//! Environment configuration. Spec: docs/technical-specs/11-environment-configuration.md.

use std::{net::SocketAddr, path::PathBuf};

use anyhow::{Context, bail};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Env {
    Dev,
    Test,
    Prod,
}

#[derive(Clone, Debug)]
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

/// A two-valued variable: unset means `no`; anything but `yes` or `no` stops the start, so a
/// typo such as `TRUE` cannot silently turn a feature off.
fn flag(value: Option<String>, key: &str, yes: &str, no: &str) -> anyhow::Result<bool> {
    // Trimmed: an env file line can carry a trailing space or CR.
    match value.as_deref().map(str::trim) {
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
        let var = |k: &str| get(k).filter(|v| !v.trim().is_empty());
        // No default: a prod host that loses this line must not come up in dev with reset-state.
        let env = match var("MONITA_ENV").as_deref() {
            None => bail!("MONITA_ENV is required: dev, test or prod"),
            Some("dev") => Env::Dev,
            Some("test") => Env::Test,
            Some("prod") => Env::Prod,
            Some(other) => bail!("MONITA_ENV must be dev, test or prod, got {other:?}"),
        };
        let bind = var("MONITA_BIND")
            .unwrap_or_else(|| "127.0.0.1:8080".into())
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

        Ok(Self {
            env,
            bind,
            base_url,
            db_path: var("MONITA_DB_PATH")
                .unwrap_or_else(|| "./data/monita.db".into())
                .into(),
            backup_dir: var("MONITA_BACKUP_DIR").map(PathBuf::from),
            backup_keep: parse_num("MONITA_BACKUP_KEEP", 7)? as usize,
            smtp_url: var("MONITA_SMTP_URL"),
            mail_from: var("MONITA_MAIL_FROM").unwrap_or_else(|| "monita@localhost".into()),
            ping_url: var("MONITA_PING_URL"),
            trust_proxy: flag(
                var("MONITA_TRUST_PROXY"),
                "MONITA_TRUST_PROXY",
                "true",
                "false",
            )?,
            digest_utc_hour,
            reset_token,
            log_filter: var("MONITA_LOG").unwrap_or_else(|| {
                if env == Env::Dev {
                    "info,monita=debug"
                } else {
                    "info"
                }
                .into()
            }),
            log_json: flag(
                var("MONITA_LOG_FORMAT"),
                "MONITA_LOG_FORMAT",
                "json",
                "pretty",
            )?,
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
    fn refuses_to_start_without_an_env() {
        assert!(cfg(&[]).is_err());
        let c = cfg(&[("MONITA_ENV", "dev")]).unwrap();
        assert_eq!(c.base_url, "http://localhost:8080");
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
            "the documented default"
        );
        assert!(
            dev("MONITA_TRUST_PROXY", "true \r").unwrap().trust_proxy,
            "env-file whitespace"
        );
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
