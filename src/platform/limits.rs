//! `/admin/settings`, the System page: email delivery, the server's configuration, the runtime
//! limits a super admin can change (AC-11.03, AC-31.03), and the fixed caps with their use.

use axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, put},
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
        .route("/api/ui/admin/time-zone", put(save_time_zone))
}

/// Each limit's group, what it does, and its value in `0001_init.sql`.
const META: &[(&str, &str, &str, i64)] = &[
    (
        "check_concurrency",
        "Monitoring",
        "How many checks Monita runs at once. When every slot is busy, the next check waits a moment. Raise it if you have many monitors and their checks run late.",
        50,
    ),
    (
        "rl_ip_per_min",
        "Visitors and apps",
        "How often one visitor can open a status page, send a bug report or try to sign in, each minute. This stops one person or bot from flooding Monita.",
        60,
    ),
    (
        "rl_key_per_min",
        "Visitors and apps",
        "How often one of your apps can call the Monita API with its key, each minute.",
        120,
    ),
    (
        "rl_ping_per_s",
        "Visitors and apps",
        "How often one heartbeat can ping Monita, each second. Extra pings in the same second are turned away; the first one still keeps the heartbeat alive.",
        1,
    ),
    (
        "tickets_per_ref_per_day",
        "Bug reports",
        "How many bug reports one user can send each day. Monita knows the user by their ref ID.",
        5,
    ),
    (
        "tickets_per_project_per_day",
        "Bug reports",
        "How many bug reports one project accepts each day, from all its users together.",
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
    mail: super::mail_api::MailView,
    email: EmailHealth,
    email_days: i64,
    server: Vec<(&'static str, String)>,
    caps: Vec<Cap>,
    /// Minutes east of UTC that every page shows times in.
    utc_offset_min: i64,
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
            "Mode",
            match c.env {
                Env::Dev => "Development",
                Env::Test => "Test",
                Env::Prod => "Production",
            }
            .into(),
        ),
        ("Address people open", c.base_url.clone()),
        ("Listens on", c.bind.to_string()),
        ("Data file", c.db_path.display().to_string()),
        (
            "Backups",
            match &c.backup_dir {
                Some(d) => format!(
                    "Every night to {}, keeping the last {}",
                    d.display(),
                    c.backup_keep
                ),
                None => "Off".into(),
            },
        ),
        (
            "Check history kept",
            format!("{} days", retention::RAW_KEEP_S / 86_400),
        ),
        (
            "Sent alert history kept",
            format!("{} days", retention::OUTBOX_KEEP_S / 86_400),
        ),
        (
            "Tells an outside service it is up",
            yes_no(c.ping_url.is_some()),
        ),
        ("Runs behind a proxy", yes_no(c.trust_proxy)),
        (
            "Daily bug report email",
            format!("{:02}:00 UTC", c.digest_utc_hour),
        ),
        (
            "Log style",
            if c.log_json { "JSON" } else { "Plain text" }.into(),
        ),
    ]
}

async fn view(state: &AppState) -> anyhow::Result<SystemView> {
    let c = state.db.read().await?;
    let current = settings::all(&c).await?;
    let utc_offset_min = settings::utc_offset(&c).await?;
    let saved_mail = settings::mail(&c).await?;
    drop(c);
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
            label: "User accounts",
            max: caps::USERS,
            used: Some(users),
        },
        Cap {
            label: "Monitors in one project",
            max: caps::MONITORS_PER_PROJECT,
            used: None,
        },
        Cap {
            label: "Heartbeats in one project",
            max: caps::HEARTBEATS_PER_PROJECT,
            used: None,
        },
        Cap {
            label: "Active API keys in one project",
            max: caps::KEYS_PER_PROJECT,
            used: None,
        },
        Cap {
            label: "Email addresses in one alert setting",
            max: caps::EMAILS_PER_ROUTE as i64,
            used: None,
        },
        Cap {
            label: "Updates on one incident",
            max: caps::UPDATES_PER_INCIDENT,
            used: None,
        },
        Cap {
            label: "Open incidents you reported by hand, in one project",
            max: caps::OPEN_MANUAL_PER_PROJECT,
            used: None,
        },
    ];
    let config = &state.config;
    Ok(SystemView {
        limits,
        groups: ["Monitoring", "Visitors and apps", "Bug reports"],
        mail: super::mail_api::view(
            saved_mail.as_ref(),
            config.smtp_url.as_deref(),
            &config.mail_from,
        ),
        email: notify::email_health(&state.db, now - EMAIL_DAYS * 86_400).await?,
        email_days: EMAIL_DAYS,
        server: server_rows(config),
        caps,
        utc_offset_min,
    })
}

async fn page(State(state): State<AppState>, _: AdminUser) -> UiResult {
    Ok(data(view(&state).await?))
}

#[derive(serde::Deserialize)]
struct TimeZoneInput {
    utc_offset_min: i64,
}

/// The offset every page shows times in. 422 outside UTC-12:00 to UTC+14:00 in quarter hours.
async fn save_time_zone(
    State(state): State<AppState>,
    _: AdminUser,
    Body(f): Body<TimeZoneInput>,
) -> UiResult {
    let min = f.utc_offset_min;
    if !settings::valid_offset(min) {
        return Err(UiError::field(
            "utc_offset_min",
            "Pick a time zone from the list.",
        ));
    }
    state
        .db
        .write(async |c| settings::set_utc_offset(c, min).await)
        .await?;
    Ok(StatusCode::NO_CONTENT.into_response())
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
