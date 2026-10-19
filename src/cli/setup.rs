//! `monita setup`: asks for the production values, writes the env file, and creates the first
//! super admin. Contract: docs/api-specs/10-cli.md 10.2.

use std::path::Path;

use anyhow::{Context, bail};

use super::{envfile, read_line, read_secret};
use crate::{
    accounts::{self, AccountError},
    platform::config::Config,
};

/// Asked in this order. Every other line of the file is kept as it is.
const QUESTIONS: [(&str, &str); 7] = [
    (
        "MONITA_BASE_URL",
        "Public URL, as people open it (https://...)",
    ),
    (
        "MONITA_BIND",
        "Listen address; only the reverse proxy may reach it",
    ),
    (
        "MONITA_TRUST_PROXY",
        "Read the client IP from X-Forwarded-For (true or false)",
    ),
    ("MONITA_DB_PATH", "Database file"),
    ("MONITA_BACKUP_DIR", "Backup folder"),
    (
        "MONITA_SMTP_URL",
        "SMTP URL, smtps://user:password@host:465",
    ),
    ("MONITA_MAIL_FROM", "Sender of alert emails"),
];

/// The production value of spec 11, or empty where it has none.
fn default(key: &str, base_url: &str) -> String {
    match key {
        "MONITA_BIND" => "127.0.0.1:7466".into(),
        "MONITA_TRUST_PROXY" => "true".into(),
        "MONITA_DB_PATH" => "/var/lib/monita/monita.db".into(),
        "MONITA_BACKUP_DIR" => "/var/backups/monita".into(),
        "MONITA_MAIL_FROM" => reqwest::Url::parse(base_url)
            .ok()
            .and_then(|u| u.host_str().map(|h| format!("Monita <monita@{h}>")))
            .unwrap_or_else(|| "monita@localhost".into()),
        _ => String::new(),
    }
}

pub async fn run(path: &Path, yes: bool) -> anyhow::Result<()> {
    let old = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e).with_context(|| format!("could not read {}", path.display())),
    };
    let existing = envfile::parse(&old);
    let in_file = |k: &str| {
        existing
            .iter()
            .rev()
            .find(|(f, _)| f == k)
            .map(|(_, v)| v.clone())
    };
    if !yes {
        eprintln!(
            "Monita setup writes {}. Enter keeps the value in brackets; - leaves it empty.",
            path.display()
        );
    }
    let mut set: Vec<(&str, String)> = vec![("MONITA_ENV", "prod".into())];
    for (key, ask) in QUESTIONS {
        let base_url = set.iter().find(|(k, _)| *k == "MONITA_BASE_URL");
        let current = in_file(key).unwrap_or_else(|| default(key, base_url.map_or("", |(_, v)| v)));
        let value = if yes {
            std::env::var(key).map_or(current, |v| v.trim().to_string())
        } else {
            eprint!("{ask} [{current}]: ");
            match read_line()?.trim() {
                "" => current,
                "-" => String::new(),
                v => v.to_string(),
            }
        };
        set.push((key, value));
    }

    // Before the file is written: a scripted setup that cannot create its admin changes nothing.
    if yes && admin_email_set() && admin_password().is_none() {
        bail!("MONITA_ADMIN_PASSWORD is required with MONITA_ADMIN_EMAIL; nothing was written");
    }
    let text = envfile::update(&old, &set)?;
    let file = envfile::parse(&text);
    let config = Config::from_lookup(|k| {
        file.iter()
            .rev()
            .find(|(f, _)| f == k)
            .map(|(_, v)| v.clone())
    })
    .with_context(|| format!("{} was not written", path.display()))?;
    if text == old {
        println!("nothing changed in {}", path.display());
    } else {
        envfile::write(path, &text)?;
        println!("wrote {}", path.display());
    }
    first_admin(&config, yes).await?;
    println!("next: sudo monita service install");
    Ok(())
}

/// Creates the first super admin. An email already taken is kept, so setup can run again.
async fn first_admin(config: &Config, yes: bool) -> anyhow::Result<()> {
    let email = if yes {
        std::env::var("MONITA_ADMIN_EMAIL").unwrap_or_default()
    } else {
        eprint!("First super admin email (blank to skip): ");
        read_line()?
    };
    let email = email.trim();
    if email.is_empty() {
        return Ok(());
    }
    let password = if yes {
        admin_password().context("MONITA_ADMIN_PASSWORD is required with MONITA_ADMIN_EMAIL")?
    } else {
        let first = read_secret(&format!("Password for {email}: "))?;
        if read_secret("The same password again: ")? != first {
            bail!("the two passwords differ; no account was created");
        }
        first
    };
    super::offline(config, async |db| {
        match accounts::create_super_admin(db, email, &password).await {
            Ok(user) => println!("created super admin {}", user.email),
            Err(AccountError::EmailTaken) => {
                println!("an account with {email} already exists; kept")
            }
            Err(e) => return Err(e.into()),
        }
        Ok(())
    })
    .await
}

fn admin_email_set() -> bool {
    std::env::var("MONITA_ADMIN_EMAIL").is_ok_and(|e| !e.trim().is_empty())
}

fn admin_password() -> Option<String> {
    std::env::var("MONITA_ADMIN_PASSWORD")
        .ok()
        .filter(|p| !p.is_empty())
}
