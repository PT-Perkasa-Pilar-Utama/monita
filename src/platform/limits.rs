//! `/admin/settings`, the System page: email delivery, the server's configuration, the runtime
//! limits a super admin can change (AC-11.03, AC-31.03), and the fixed caps with their use.

use axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, put},
};
use serde::Serialize;
use std::collections::HashMap;

use super::{
    caps,
    config::{Config, Env},
    now, settings,
    ui::{Body, UiError, UiResult, data},
};
use crate::{
    accounts::{self, AdminUser},
    app::AppState,
    checks::retention,
    notify::{self, EmailHealth},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/ui/admin/system", get(page))
        .route("/api/ui/admin/limits", put(save))
        .route("/api/ui/admin/test-email", post(test_email))
}

/// Each limit's group, what it does, and its value in `0001_init.sql`.
const META: &[(&str, &str, &str, i64)] = &[
    (
        "check_concurrency",
        "Checks",
        "Monita runs at most this many checks at the same time. A check that finds no free slot waits for the next one.",
        50,
    ),
    (
        "rl_ip_per_min",
        "Rate limits",
        "One visitor IP can load public pages, send the ticket form or try to sign in this many times a minute.",
        60,
    ),
    (
        "rl_key_per_min",
        "Rate limits",
        "One API key can call the API this many times a minute.",
        120,
    ),
    (
        "rl_ping_per_s",
        "Rate limits",
        "A heartbeat accepts this many pings a second. Extra pings get 429 and are not stored.",
        1,
    ),
    (
        "tickets_per_ref_per_day",
        "Tickets",
        "One reference ID can send this many tickets a day.",
        5,
    ),
    (
        "tickets_per_project_per_day",
        "Tickets",
        "One project accepts this many tickets a day from all its users.",
        100,
    ),
];

/// The window the email counts cover.
const EMAIL_DAYS: i64 = 7;

#[derive(Serialize)]
pub struct Limit {
    pub key: &'static str,
    pub label: &'static str,
    pub group: &'static str,
    pub help: &'static str,
    pub value: i64,
    pub default: i64,
    pub max: i64,
    /// Where the value and the default sit on a log scale from 1 to `max`, 0 to 100.
    pub at: f64,
    pub default_at: f64,
}

/// The mail server as the page shows it. The password never leaves `smtp_view`.
#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct Mail {
    pub server: Option<String>,
    pub user: Option<String>,
    pub from: String,
}

#[derive(Serialize)]
pub struct Cap {
    pub label: &'static str,
    pub max: i64,
    /// How many exist now, for the caps that count the whole instance.
    pub used: Option<i64>,
}

/// Everything the System page shows.
#[derive(Serialize)]
struct SystemView {
    limits: Vec<Limit>,
    groups: [&'static str; 3],
    mail: Mail,
    email: EmailHealth,
    email_days: i64,
    server: Vec<(&'static str, String)>,
    caps: Vec<Cap>,
}

/// A log scale, so 1, 60 and 1 000 000 all land somewhere visible on one bar.
pub fn position(value: i64, max: i64) -> f64 {
    let v = (value.max(1) as f64).ln();
    (v / (max.max(2) as f64).ln() * 100.0).clamp(0.0, 100.0)
}

/// `smtps://user:secret@mail.example.com:465` shows as the server and the user, never the secret.
pub fn smtp_view(url: Option<&str>, from: &str) -> Mail {
    let parsed = url.and_then(|u| reqwest::Url::parse(u).ok());
    Mail {
        server: parsed.as_ref().map(|u| match (u.host_str(), u.port()) {
            (Some(h), Some(p)) => format!("{}://{h}:{p}", u.scheme()),
            (Some(h), None) => format!("{}://{h}", u.scheme()),
            _ => u.scheme().to_string(),
        }),
        user: parsed
            .as_ref()
            .map(|u| u.username().to_string())
            .filter(|u| !u.is_empty()),
        from: from.to_string(),
    }
}

fn server_rows(c: &Config) -> Vec<(&'static str, String)> {
    let yes_no = |b: bool| if b { "Yes" } else { "No" }.to_string();
    vec![
        (
            "Environment",
            match c.env {
                Env::Dev => "dev",
                Env::Test => "test",
                Env::Prod => "prod",
            }
            .into(),
        ),
        ("Public URL", c.base_url.clone()),
        ("Listens on", c.bind.to_string()),
        ("Database file", c.db_path.display().to_string()),
        (
            "Backups",
            match &c.backup_dir {
                Some(d) => format!("Nightly to {}, keeping {}", d.display(), c.backup_keep),
                None => "Off".into(),
            },
        ),
        (
            "Single checks kept",
            format!("{} days", retention::RAW_KEEP_S / 86_400),
        ),
        (
            "Sent alerts kept",
            format!("{} days", retention::OUTBOX_KEEP_S / 86_400),
        ),
        ("Pings an outside watcher", yes_no(c.ping_url.is_some())),
        ("Behind a proxy", yes_no(c.trust_proxy)),
        ("Daily digest", format!("{:02}:00 UTC", c.digest_utc_hour)),
        (
            "Log format",
            if c.log_json { "JSON" } else { "Text" }.into(),
        ),
    ]
}

async fn view(state: &AppState) -> anyhow::Result<SystemView> {
    let current = settings::all(&state.db.read().await?).await?;
    let limits = META
        .iter()
        .map(|&(key, group, help, default)| {
            let label = settings::KEYS
                .iter()
                .find(|(k, _)| *k == key)
                .map_or(key, |(_, l)| *l);
            let value = current
                .iter()
                .find(|(k, _)| k == key)
                .map_or(default, |(_, v)| *v);
            let max = settings::max_for(key);
            Limit {
                key,
                label,
                group,
                help,
                at: position(value, max),
                default_at: position(default, max),
                value,
                default,
                max,
            }
        })
        .collect();
    let now = now();
    let users = accounts::list_users(&state.db).await?.len() as i64;
    let caps = vec![
        Cap {
            label: "Accounts",
            max: caps::USERS,
            used: Some(users),
        },
        Cap {
            label: "Monitors per project",
            max: caps::MONITORS_PER_PROJECT,
            used: None,
        },
        Cap {
            label: "Heartbeats per project",
            max: caps::HEARTBEATS_PER_PROJECT,
            used: None,
        },
        Cap {
            label: "Live API keys per project",
            max: caps::KEYS_PER_PROJECT,
            used: None,
        },
        Cap {
            label: "Addresses per alert route",
            max: caps::EMAILS_PER_ROUTE as i64,
            used: None,
        },
        Cap {
            label: "Updates per incident",
            max: caps::UPDATES_PER_INCIDENT,
            used: None,
        },
        Cap {
            label: "Open reported incidents per project",
            max: caps::OPEN_MANUAL_PER_PROJECT,
            used: None,
        },
    ];
    let config = &state.config;
    Ok(SystemView {
        limits,
        groups: ["Checks", "Rate limits", "Tickets"],
        mail: smtp_view(config.smtp_url.as_deref(), &config.mail_from),
        email: notify::email_health(&state.db, now - EMAIL_DAYS * 86_400).await?,
        email_days: EMAIL_DAYS,
        server: server_rows(config),
        caps,
    })
}

async fn page(State(state): State<AppState>, _: AdminUser) -> UiResult {
    Ok(data(view(&state).await?))
}

/// Queues a test email to the signed-in admin through the outbox, like any alert.
async fn test_email(State(state): State<AppState>, AdminUser(admin): AdminUser) -> UiResult {
    let to = admin.email.clone();
    let body = format!(
        "This is a test email from Monita at {}. If you can read it, email alerts reach you.",
        state.config.base_url
    );
    let at = now();
    state
        .db
        .write(async |c| notify::queue_email(c, &to, "Monita test email", &body, at).await)
        .await?;
    Ok(data(serde_json::json!({ "sent_to": to })))
}

/// AC-11.03, AC-31.03. Every limit must be a whole number from 1 to its maximum, or nothing is
/// saved and each bad one is 422 on its key.
async fn save(
    State(state): State<AppState>,
    _: AdminUser,
    Body(form): Body<HashMap<String, serde_json::Value>>,
) -> UiResult {
    let mut parsed = Vec::new();
    let mut errors = std::collections::BTreeMap::new();
    for (key, label) in settings::KEYS {
        let max = settings::max_for(key);
        let value = form.get(*key).and_then(|v| match v {
            serde_json::Value::Number(n) => n.as_i64(),
            serde_json::Value::String(s) => s.trim().parse().ok(),
            _ => None,
        });
        match value.filter(|v| (1..=max).contains(v)) {
            Some(v) => parsed.push((*key, v)),
            None => {
                errors.insert(
                    key.to_string(),
                    format!("{label}: enter a whole number from 1 to {max}."),
                );
            }
        }
    }
    if !errors.is_empty() {
        return Err(UiError::fields(errors));
    }
    state
        .db
        .write(async |c| {
            for (key, value) in &parsed {
                settings::set(c, key, *value).await?;
            }
            Ok(())
        })
        .await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_limit_has_a_group_and_the_default_the_migration_writes() {
        let sql = include_str!("../../migrations/0001_init.sql");
        assert_eq!(META.len(), settings::KEYS.len());
        for (key, _) in settings::KEYS {
            let (_, _, _, default) = META.iter().find(|m| m.0 == *key).expect("a META row");
            assert!(sql.contains(&format!("('{key}', '{default}')")), "{key}");
        }
    }

    #[test]
    fn the_mail_view_never_carries_the_password() {
        let m = smtp_view(
            Some("smtps://alerts:hunter2@mail.example.com:465"),
            "Monita <m@x.co>",
        );
        assert_eq!(m.server.as_deref(), Some("smtps://mail.example.com:465"));
        assert_eq!(m.user.as_deref(), Some("alerts"));
        assert!(!format!("{m:?}").contains("hunter2"));
        assert_eq!(smtp_view(None, "m@x.co").server, None);
    }

    #[test]
    fn the_scale_puts_one_at_the_start_and_the_cap_at_the_end() {
        assert_eq!(position(1, 1_000_000), 0.0);
        assert_eq!(position(1_000_000, 1_000_000), 100.0);
        assert!(position(60, 1_000_000) > position(5, 1_000_000));
    }
}
