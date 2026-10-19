//! The Email page's mail server form: save it, clear it so `MONITA_SMTP_URL` or the log applies,
//! test the connection, and send a test email.

use std::collections::BTreeMap;

use axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{post, put},
};
use serde::Deserialize;

use super::{
    settings::{self, MailSettings},
    ui::{Body, UiError, UiResult, data},
};
use crate::{accounts::AdminUser, app::AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/ui/admin/mail", put(save).delete(clear))
        .route("/api/ui/admin/mail/test-connection", post(test_connection))
        .route("/api/ui/admin/mail/test-send", post(test_send))
}

/// The form as sent. A blank password keeps the saved one; `remove_password` clears it.
#[derive(Deserialize, Default)]
#[serde(default)]
pub struct MailInput {
    pub host: String,
    pub port: String,
    pub security: String,
    pub user: String,
    pub password: String,
    pub remove_password: bool,
    pub from: String,
    pub skip_verify: bool,
}

impl MailInput {
    /// The settings to save, or each refused field with what to do. `saved` gives the password
    /// a blank field keeps.
    pub fn settings(
        &self,
        saved: Option<&MailSettings>,
    ) -> Result<MailSettings, BTreeMap<String, String>> {
        let mut e = BTreeMap::new();
        let mut bad = |k: &str, m: &str| {
            e.entry(k.to_string()).or_insert_with(|| m.to_string());
        };
        let host = self.host.trim();
        if host.is_empty() || host.contains(char::is_whitespace) || host.contains("://") {
            bad(
                "host",
                "Enter the server's name, for example mail.company.co.id.",
            );
        }
        let port = self.port.trim().parse::<u16>().ok().filter(|p| *p > 0);
        if port.is_none() {
            bad("port", "Enter a port from 1 to 65535.");
        }
        if !["tls", "starttls", "none"].contains(&self.security.as_str()) {
            bad("security", "Pick how Monita connects.");
        }
        let user = Some(self.user.trim().to_string()).filter(|u| !u.is_empty());
        let password = if self.remove_password {
            None
        } else if self.password.is_empty() {
            saved.and_then(|s| s.password.clone())
        } else {
            Some(self.password.clone())
        };
        if password.is_some() && user.is_none() {
            bad("user", "Enter the user name that goes with the password.");
        }
        let from = self.from.trim();
        if from.parse::<lettre::message::Mailbox>().is_err() {
            bad(
                "from",
                "Enter an address, for example Monita <monita@company.co.id>.",
            );
        }
        if !e.is_empty() {
            return Err(e);
        }
        Ok(MailSettings {
            host: host.to_string(),
            port: port.unwrap_or(587),
            security: self.security.clone(),
            user,
            password,
            from: from.to_string(),
            skip_verify: self.skip_verify && self.security != "none",
        })
    }
}

/// The mail server as the System page shows it: which source applies, and the saved fields.
/// The password never leaves the server; `has_password` says whether one is saved.
#[derive(serde::Serialize)]
pub struct MailView {
    /// `saved` (this page), `env` (`MONITA_SMTP_URL`) or `none` (the log).
    pub source: &'static str,
    /// The server as `scheme://host:port`, for any source.
    pub server: Option<String>,
    pub user: Option<String>,
    pub from: String,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub security: Option<String>,
    pub has_password: bool,
    pub skip_verify: bool,
}

pub fn view(saved: Option<&MailSettings>, smtp_url: Option<&str>, env_from: &str) -> MailView {
    match saved {
        Some(m) => MailView {
            source: "saved",
            server: Some(format!(
                "{}://{}:{}",
                if m.security == "tls" { "smtps" } else { "smtp" },
                m.host,
                m.port
            )),
            user: m.user.clone(),
            from: m.from.clone(),
            host: Some(m.host.clone()),
            port: Some(m.port),
            security: Some(m.security.clone()),
            has_password: m.password.is_some(),
            skip_verify: m.skip_verify,
        },
        None => {
            let env = super::limits::smtp_view(smtp_url, env_from);
            MailView {
                source: if env.server.is_some() { "env" } else { "none" },
                server: env.server,
                user: env.user,
                from: env.from,
                host: None,
                port: None,
                security: None,
                has_password: false,
                skip_verify: false,
            }
        }
    }
}

async fn save(State(state): State<AppState>, _: AdminUser, Body(f): Body<MailInput>) -> UiResult {
    let saved = settings::mail(&state.db.read().await?).await?;
    let m = f.settings(saved.as_ref()).map_err(UiError::fields)?;
    state
        .db
        .write(async |c| settings::set_mail(c, Some(&m)).await)
        .await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

/// Tries the form's values, saved or not: connect, start TLS, sign in. 200 either way; the
/// body says whether it connected and, if not, why.
async fn test_connection(
    State(state): State<AppState>,
    _: AdminUser,
    Body(f): Body<MailInput>,
) -> UiResult {
    let saved = settings::mail(&state.db.read().await?).await?;
    let m = f.settings(saved.as_ref()).map_err(UiError::fields)?;
    Ok(data(match crate::notify::test_connection(&m).await {
        Ok(()) => serde_json::json!({ "ok": true, "server": format!("{}:{}", m.host, m.port) }),
        Err(e) => serde_json::json!({ "ok": false, "error": format!("{e:#}") }),
    }))
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct TestSendInput {
    to: String,
}

/// Sends one email now through the mail server in use. 200 either way; the body says whether it
/// went, through which source, and the error if not.
async fn test_send(
    State(state): State<AppState>,
    _: AdminUser,
    Body(f): Body<TestSendInput>,
) -> UiResult {
    let to = f.to.trim().to_string();
    if to.parse::<lettre::Address>().is_err() {
        return Err(UiError::field(
            "to",
            "Enter an email address, for example you@company.co.id.",
        ));
    }
    let saved = settings::mail(&state.db.read().await?).await?;
    let source = view(
        saved.as_ref(),
        state.config.smtp_url.as_deref(),
        &state.config.mail_from,
    )
    .source;
    let email = crate::notify::SentEmail {
        to: to.clone(),
        subject: "Monita test email".into(),
        body: format!(
            "This is a test email from Monita at {}. If you can read it, alerts reach you.",
            state.config.base_url
        ),
    };
    Ok(data(match state.mailer.send_now(&state.db, email).await {
        Ok(()) => serde_json::json!({ "ok": true, "to": to, "source": source }),
        Err(e) => {
            serde_json::json!({ "ok": false, "to": to, "source": source, "error": format!("{e:#}") })
        }
    }))
}

async fn clear(State(state): State<AppState>, _: AdminUser) -> UiResult {
    state
        .db
        .write(async |c| settings::set_mail(c, None).await)
        .await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn form() -> MailInput {
        MailInput {
            host: " mail.company.co.id ".into(),
            port: "587".into(),
            security: "starttls".into(),
            user: "monita".into(),
            password: String::new(),
            from: "Monita <monita@company.co.id>".into(),
            ..MailInput::default()
        }
    }

    #[test]
    fn a_blank_password_keeps_the_saved_one_and_remove_clears_it() {
        let saved = MailSettings {
            password: Some("hunter2".into()),
            ..form().settings(None).unwrap()
        };
        assert_eq!(
            form().settings(Some(&saved)).unwrap().password.as_deref(),
            Some("hunter2")
        );
        let removed = MailInput {
            remove_password: true,
            ..form()
        };
        assert_eq!(removed.settings(Some(&saved)).unwrap().password, None);
        let new = MailInput {
            password: "s3cret".into(),
            ..form()
        };
        assert_eq!(
            new.settings(Some(&saved)).unwrap().password.as_deref(),
            Some("s3cret")
        );
        assert_eq!(form().settings(None).unwrap().host, "mail.company.co.id");
    }

    #[test]
    fn each_bad_field_is_named() {
        let bad = MailInput {
            host: "smtp://x".into(),
            port: "70000".into(),
            security: "ssl".into(),
            user: String::new(),
            password: "p".into(),
            from: "not an address".into(),
            ..MailInput::default()
        };
        let e = bad.settings(None).unwrap_err();
        let keys: Vec<&str> = e.keys().map(String::as_str).collect();
        assert_eq!(keys, ["from", "host", "port", "security", "user"]);
    }
}
