//! The mail port: the mail server an admin saved, else `MONITA_SMTP_URL`, else the log;
//! memory in tests.

use std::sync::{Arc, Mutex};

use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    message::{Attachment, Mailbox, MultiPart, SinglePart, header::ContentType},
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
    pub attachment: Option<super::Pdf>,
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

/// Connects to the mail server and signs in, without sending. The error says what failed.
pub async fn test_connection(m: &MailSettings) -> anyhow::Result<()> {
    let t = transport(m)?;
    match tokio::time::timeout(SEND_TIMEOUT, t.test_connection()).await {
        Ok(Ok(true)) => Ok(()),
        Ok(Ok(false)) => anyhow::bail!("the server answered, but not as a mail server"),
        Ok(Err(e)) => Err(e.into()),
        Err(_) => anyhow::bail!("no answer within {} seconds", SEND_TIMEOUT.as_secs()),
    }
}

impl Mailer {
    /// Sends one email now, outside the outbox, for the admin's test. The error says what failed.
    pub async fn send_now(&self, db: &Db, email: SentEmail) -> anyhow::Result<()> {
        match tokio::time::timeout(SEND_TIMEOUT, self.send(db, email)).await {
            Ok(r) => r,
            Err(_) => anyhow::bail!("no answer within {} seconds", SEND_TIMEOUT.as_secs()),
        }
    }

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
        transport.send(message(from, email)?).await?;
        Ok(())
    }
}

/// Plain text, or plain text and a PDF in a `multipart/mixed` message (v1.2 Q59).
fn message(from: Mailbox, email: SentEmail) -> anyhow::Result<Message> {
    let head = Message::builder()
        .from(from)
        .to(email.to.parse()?)
        .subject(email.subject);
    Ok(match email.attachment {
        None => head.body(email.body)?,
        Some(pdf) => head.multipart(
            MultiPart::mixed()
                .singlepart(SinglePart::plain(email.body))
                .singlepart(
                    Attachment::new(pdf.name)
                        .body(pdf.bytes, ContentType::parse("application/pdf")?),
                ),
        )?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_report_email_carries_its_pdf_as_an_attachment() {
        let email = |attachment| SentEmail {
            to: "boss@x.co".into(),
            subject: "Payments weekly report".into(),
            body: "Attached.".into(),
            attachment,
        };
        let from: Mailbox = "Monita <monita@x.co>".parse().unwrap();
        let pdf = super::super::Pdf {
            name: "payments-2026-11-16-to-2026-11-22.pdf".into(),
            bytes: b"%PDF-1.4 x".to_vec(),
        };
        let raw = String::from_utf8(message(from.clone(), email(Some(pdf))).unwrap().formatted())
            .unwrap();
        assert!(raw.contains("multipart/mixed"), "{raw}");
        assert!(raw.contains("Content-Type: application/pdf"));
        // lettre writes the name as an RFC 2231 parameter: `filename*0="..."`.
        assert!(raw.contains("Content-Disposition: attachment;"));
        assert!(
            raw.contains("\"payments-2026-11-16-to-2026-11-22.pdf\""),
            "{raw}"
        );
        let plain = String::from_utf8(message(from, email(None)).unwrap().formatted()).unwrap();
        assert!(!plain.contains("multipart"));
    }

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
