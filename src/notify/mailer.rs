//! The mail port: the mail server an admin saved, else `MONITA_SMTP_URL`, else the log;
//! memory in tests.

use std::sync::{Arc, Mutex};

use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    message::Mailbox,
    transport::smtp::{
        authentication::Credentials,
        client::{Tls, TlsParameters},
    },
};

use super::delivery::SEND_TIMEOUT;
use crate::platform::{
    config::Config,
    db::Db,
    settings::{self, MailSettings},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SentEmail {
    pub to: String,
    pub subject: String,
    pub body: String,
}

/// `MONITA_SMTP_URL` and `MONITA_MAIL_FROM`, built once at start.
#[derive(Clone)]
pub struct EnvMail {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: Mailbox,
}

/// The mail port. `Live` reads the saved settings on each send, so a save on the System page
/// applies to the next email. `Memory` is for tests.
#[derive(Clone)]
pub enum Mailer {
    Live { env: Option<EnvMail> },
    Memory(Arc<Mutex<Vec<SentEmail>>>),
}

/// The lettre transport for a saved mail server. No network until a send.
pub fn transport(m: &MailSettings) -> anyhow::Result<AsyncSmtpTransport<Tokio1Executor>> {
    let tls = || -> anyhow::Result<TlsParameters> {
        Ok(TlsParameters::builder(m.host.clone())
            .dangerous_accept_invalid_certs(m.skip_verify)
            .build_rustls()?)
    };
    let mut b = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(m.host.as_str())
        .port(m.port)
        .timeout(Some(SEND_TIMEOUT));
    b = match m.security.as_str() {
        "tls" => b.tls(Tls::Wrapper(tls()?)),
        "starttls" => b.tls(Tls::Required(tls()?)),
        _ => b.tls(Tls::None),
    };
    if let Some(user) = &m.user {
        b = b.credentials(Credentials::new(
            user.clone(),
            m.password.clone().unwrap_or_default(),
        ));
    }
    Ok(b.build())
}

impl Mailer {
    pub fn from_config(config: &Config) -> anyhow::Result<Self> {
        let env = match &config.smtp_url {
            Some(url) => Some(EnvMail {
                transport: AsyncSmtpTransport::<Tokio1Executor>::from_url(url)?
                    .timeout(Some(SEND_TIMEOUT))
                    .build(),
                from: config.mail_from.parse()?,
            }),
            None => None,
        };
        Ok(Mailer::Live { env })
    }

    pub fn memory() -> Self {
        Mailer::Memory(Arc::default())
    }

    pub fn sent(&self) -> Vec<SentEmail> {
        match self {
            Mailer::Memory(m) => m.lock().expect("mailer lock").clone(),
            Mailer::Live { .. } => Vec::new(),
        }
    }

    pub(super) async fn send(&self, db: &Db, email: SentEmail) -> anyhow::Result<()> {
        let env = match self {
            Mailer::Memory(m) => {
                m.lock().expect("mailer lock").push(email);
                return Ok(());
            }
            Mailer::Live { env } => env,
        };
        let saved = settings::mail(&db.read().await?).await?;
        let (transport, from) = match (saved, env) {
            (Some(m), _) => (transport(&m)?, m.from.parse::<Mailbox>()?),
            (None, Some(e)) => (e.transport.clone(), e.from.clone()),
            (None, None) => {
                tracing::info!(to = %email.to, subject = %email.subject, "email (no mail server set)");
                return Ok(());
            }
        };
        let msg = Message::builder()
            .from(from)
            .to(email.to.parse()?)
            .subject(email.subject)
            .body(email.body)?;
        transport.send(msg).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn saved(security: &str, user: Option<&str>) -> MailSettings {
        MailSettings {
            host: "mail.company.co.id".into(),
            port: 587,
            security: security.into(),
            user: user.map(str::to_string),
            password: Some("secret".into()),
            from: "Monita <monita@company.co.id>".into(),
            skip_verify: security == "tls",
        }
    }

    /// The SMTP pool needs a Tokio runtime, as in `monita serve`.
    #[tokio::test]
    async fn every_security_choice_and_the_env_url_build_a_transport() {
        for security in ["tls", "starttls", "none"] {
            assert!(
                transport(&saved(security, Some("monita"))).is_ok(),
                "{security}"
            );
        }
        assert!(transport(&saved("starttls", None)).is_ok(), "no user");
        let config = Config::from_lookup(|k| match k {
            "MONITA_ENV" => Some("prod".into()),
            "MONITA_BASE_URL" => Some("https://monita.company.co.id".into()),
            "MONITA_SMTP_URL" => Some("smtps://user:password@smtp.company.co.id:465".into()),
            "MONITA_MAIL_FROM" => Some("Monita <monita@company.co.id>".into()),
            _ => None,
        })
        .unwrap();
        assert!(matches!(
            Mailer::from_config(&config).unwrap(),
            Mailer::Live { env: Some(_) }
        ));
        let no_smtp = Config::from_lookup(|k| (k == "MONITA_ENV").then(|| "dev".into())).unwrap();
        assert!(matches!(
            Mailer::from_config(&no_smtp).unwrap(),
            Mailer::Live { env: None }
        ));
    }
}
