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

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Self::from_lookup(|k| std::env::var(k).ok())
    }

    /// Reads every variable through `get`, so tests pass a map instead of the process env.
    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> anyhow::Result<Self> {
        let var = |k: &str| get(k).filter(|v| !v.trim().is_empty());
        let env = match var("MONITA_ENV").as_deref() {
            None | Some("dev") => Env::Dev,
            Some("test") => Env::Test,
            Some("prod") => Env::Prod,
            Some(other) => bail!("MONITA_ENV must be dev, test or prod, got {other:?}"),
        };
        let bind = var("MONITA_BIND")
            .unwrap_or_else(|| "127.0.0.1:8080".into())
            .parse()
            .context("MONITA_BIND must be host:port")?;
        let base_url = var("MONITA_BASE_URL")
            .unwrap_or_else(|| format!("http://localhost:{}", SocketAddr::port(&bind)));
        let base_url = base_url.trim_end_matches('/').to_string();
        if env == Env::Prod && !base_url.starts_with("https://") {
            bail!("MONITA_BASE_URL must start with https:// when MONITA_ENV=prod");
        }
        let parse_num = |k: &str, default: u64| -> anyhow::Result<u64> {
            var(k).map_or(Ok(default), |v| {
                v.parse().with_context(|| format!("{k} must be a number"))
            })
        };
        let digest_utc_hour = parse_num("MONITA_DIGEST_UTC_HOUR", 1)? as u32;
        if digest_utc_hour > 23 {
            bail!("MONITA_DIGEST_UTC_HOUR must be 0 to 23");
        }
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
            trust_proxy: var("MONITA_TRUST_PROXY").as_deref() == Some("true"),
            digest_utc_hour,
            reset_token,
            log_filter: var("MONITA_LOG").unwrap_or_else(|| "info".into()),
            log_json: var("MONITA_LOG_FORMAT").as_deref() == Some("json"),
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
    fn defaults_to_dev() {
        let c = cfg(&[]).unwrap();
        assert_eq!(c.env, Env::Dev);
        assert_eq!(c.base_url, "http://localhost:8080");
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
