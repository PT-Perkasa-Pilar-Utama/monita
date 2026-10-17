//! The mail port: SMTP in production, the log without SMTP, memory in tests.

use std::sync::{Arc, Mutex};

use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor, message::Mailbox};

use super::delivery::SEND_TIMEOUT;
use crate::platform::config::Config;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SentEmail {
    pub to: String,
    pub subject: String,
    pub body: String,
}

/// The mail port. `Smtp` in production, `Log` when no SMTP server is set, `Memory` for tests.
#[derive(Clone)]
pub enum Mailer {
    Smtp {
        transport: AsyncSmtpTransport<Tokio1Executor>,
        from: Mailbox,
    },
    Log,
    Memory(Arc<Mutex<Vec<SentEmail>>>),
}

impl Mailer {
    pub fn from_config(config: &Config) -> anyhow::Result<Self> {
        let Some(url) = &config.smtp_url else {
            return Ok(Mailer::Log);
        };
        let transport = AsyncSmtpTransport::<Tokio1Executor>::from_url(url)?
            .timeout(Some(SEND_TIMEOUT))
            .build();
        Ok(Mailer::Smtp {
            transport,
            from: config.mail_from.parse()?,
        })
    }

    pub fn memory() -> Self {
        Mailer::Memory(Arc::default())
    }

    pub fn sent(&self) -> Vec<SentEmail> {
        match self {
            Mailer::Memory(m) => m.lock().expect("mailer lock").clone(),
            _ => Vec::new(),
        }
    }

    pub(super) async fn send(&self, email: SentEmail) -> anyhow::Result<()> {
        match self {
            Mailer::Smtp { transport, from } => {
                let msg = Message::builder()
                    .from(from.clone())
                    .to(email.to.parse()?)
                    .subject(email.subject)
                    .body(email.body)?;
                transport.send(msg).await?;
                Ok(())
            }
            Mailer::Log => {
                tracing::info!(to = %email.to, subject = %email.subject, "email (no SMTP configured)");
                Ok(())
            }
            Mailer::Memory(m) => {
                m.lock().expect("mailer lock").push(email);
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The SMTP pool needs a Tokio runtime, as in `monita serve`.
    #[tokio::test]
    async fn production_smtp_settings_parse() {
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
            Mailer::Smtp { .. }
        ));
        let no_smtp = Config::from_lookup(|k| (k == "MONITA_ENV").then(|| "dev".into())).unwrap();
        assert!(matches!(
            Mailer::from_config(&no_smtp).unwrap(),
            Mailer::Log
        ));
    }
}
