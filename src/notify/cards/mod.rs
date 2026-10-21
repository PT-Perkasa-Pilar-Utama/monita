//! Alert cards (v1.1 Q34). A project has two fixed cards that email its owner, one for critical
//! and one for warning, and up to `caps::ALERT_CARDS` of its own: an email list or a webhook in
//! one format, each listening to critical, warning or both, each with its own switch.
//! Contract: docs/api-specs/06-notify.md section 6.1.

pub mod api;
mod store;

use turso::Connection;

pub use store::AlertCard;
use store::Saved;

use super::{AlertError, compose::Composed, format, telegram};
use crate::platform::{caps, db::Db, now};

/// What the card form sends.
pub struct CardForm<'a> {
    pub kind: &'a str,
    pub name: &'a str,
    pub critical: bool,
    pub warning: bool,
    /// An email card's addresses, one a line or comma-separated.
    pub emails: &'a str,
    pub url: &'a str,
    pub key: &'a str,
}

/// The form as it will be stored, or the first thing wrong with it.
fn check(f: &CardForm<'_>) -> Result<Saved, AlertError> {
    let kind = f.kind.trim();
    if kind == "owner_email" {
        return Err(AlertError::Fixed);
    }
    if kind != "email" && !format::FORMATS.contains(&kind) {
        return Err(AlertError::BadFormat);
    }
    let name = f.name.trim();
    if name.is_empty() || name.chars().count() > 60 {
        return Err(AlertError::NoName);
    }
    if !f.critical && !f.warning {
        return Err(AlertError::NoLevel);
    }
    let (emails, url, key) = if kind == "email" {
        let mut emails = Vec::new();
        for e in store::split_emails(f.emails) {
            let e = crate::accounts::normalize_email(&e).map_err(|_| AlertError::BadEmail(e))?;
            if !emails.contains(&e) {
                emails.push(e);
            }
        }
        if emails.is_empty() {
            return Err(AlertError::NoEmails);
        }
        if emails.len() > caps::EMAILS_PER_CARD {
            return Err(AlertError::TooManyEmails);
        }
        (emails, None, None)
    } else {
        let (url, key) = format::check(kind, f.url, f.key)?;
        // A webhook card exists to post somewhere.
        let url = url.ok_or(AlertError::BadWebhook)?;
        (Vec::new(), Some(url), key)
    };
    Ok(Saved {
        kind: kind.to_string(),
        name: name.to_string(),
        critical: f.critical,
        warning: f.warning,
        emails,
        url,
        key,
    })
}

/// Checks the form, and that the global Telegram bot is set for a card that uses it.
async fn checked(db: &Db, f: &CardForm<'_>) -> Result<Saved, AlertError> {
    let saved = check(f)?;
    if saved.kind == "telegram_bot" && telegram::bot_name(db).await?.is_none() {
        return Err(AlertError::NoBot);
    }
    Ok(saved)
}

pub async fn list(db: &Db, project_id: i64) -> anyhow::Result<Vec<AlertCard>> {
    store::list(&db.read().await?, project_id).await
}

/// A new card, on. Returns its id.
pub async fn create(db: &Db, project_id: i64, f: &CardForm<'_>) -> Result<i64, AlertError> {
    let saved = checked(db, f).await?;
    db.write(async |c| {
        if store::own_count(c, project_id).await? >= caps::ALERT_CARDS {
            return Ok(Err(AlertError::Full));
        }
        Ok(Ok(store::insert(c, project_id, &saved).await?))
    })
    .await?
}

/// Rejects the owner's cards and a card of another project.
async fn own_card(c: &Connection, project_id: i64, id: i64) -> Result<(), AlertError> {
    match store::one(c, project_id, id).await? {
        None => Err(AlertError::NotFound),
        Some(card) if card.fixed => Err(AlertError::Fixed),
        Some(_) => Ok(()),
    }
}

pub async fn update(db: &Db, project_id: i64, id: i64, f: &CardForm<'_>) -> Result<(), AlertError> {
    let saved = checked(db, f).await?;
    db.write(async |c| {
        if let Err(e) = own_card(c, project_id, id).await {
            return Ok(Err(e));
        }
        store::update(c, project_id, id, &saved).await?;
        Ok(Ok(()))
    })
    .await?
}

pub async fn delete(db: &Db, project_id: i64, id: i64) -> Result<(), AlertError> {
    db.write(async |c| {
        if let Err(e) = own_card(c, project_id, id).await {
            return Ok(Err(e));
        }
        store::delete(c, project_id, id).await?;
        Ok(Ok(()))
    })
    .await?
}

/// Every card has a switch, the owner's too. Off keeps the card's settings.
pub async fn set_enabled(db: &Db, project_id: i64, id: i64, on: bool) -> Result<(), AlertError> {
    let found = db
        .write(async |c| store::set_enabled(c, project_id, id, on).await)
        .await?;
    if found {
        Ok(())
    } else {
        Err(AlertError::NotFound)
    }
}

/// A new project's two owner cards. Runs inside the caller's write.
pub async fn create_owner_cards(c: &Connection, project_id: i64) -> anyhow::Result<()> {
    store::create_owner_cards(c, project_id).await
}

/// Queues one alert to every card that is on and listens to `severity`. An address in several
/// cards gets one email. False when no card listens: the alert then goes nowhere, which is no
/// error, and the check, ping or incident that caused it goes on as usual.
pub(super) async fn queue_alert(
    c: &Connection,
    project_id: i64,
    slug: &str,
    severity: &str,
    msg: &Composed,
) -> anyhow::Result<bool> {
    let cards = store::listening(c, project_id, severity).await?;
    let at = now();
    let mut emails: Vec<String> = Vec::new();
    for card in &cards {
        match card.kind.as_str() {
            "owner_email" => emails.extend(crate::projects::owner_email(c, project_id).await?),
            "email" => emails.extend(card.emails.iter().cloned()),
            _ => queue_webhook(c, card, slug, msg, at).await?,
        }
    }
    emails.sort();
    emails.dedup();
    for to in &emails {
        super::store::queue(c, "email", to, Some(&msg.subject), &msg.text, at).await?;
    }
    Ok(!cards.is_empty())
}

/// One webhook card's call for one alert, in the card's format.
async fn queue_webhook(
    c: &Connection,
    card: &AlertCard,
    slug: &str,
    msg: &Composed,
    at: i64,
) -> anyhow::Result<()> {
    let Some(url) = card.url.as_deref() else {
        return Ok(());
    };
    super::queue_webhook(c, &card.kind, url, card.key.as_deref(), slug, msg, at).await
}

/// The card's test button: one test message to that card alone, on or off. Returns how many
/// recipients it went to.
pub async fn send_test(
    db: &Db,
    project_id: i64,
    id: i64,
    base_url: &str,
) -> Result<usize, AlertError> {
    db.write(async |c| {
        let (Some(card), Some((name, slug))) = (
            store::one(c, project_id, id).await?,
            crate::projects::name_slug(c, project_id).await?,
        ) else {
            return Ok(Err(AlertError::NotFound));
        };
        let at = now();
        let mut msg = super::compose::test_alert(&name, &slug, &card.name, base_url, at);
        let emails: Vec<String> = match card.kind.as_str() {
            "owner_email" => crate::projects::owner_email(c, project_id)
                .await?
                .into_iter()
                .collect(),
            "email" => card.emails.clone(),
            _ => {
                queue_webhook(c, &card, &slug, &msg, at).await?;
                // A PagerDuty test opens an incident there; resolve it at once.
                if card.kind == "pagerduty" {
                    msg.resolves = true;
                    queue_webhook(c, &card, &slug, &msg, at).await?;
                }
                return Ok(Ok(1));
            }
        };
        for to in &emails {
            super::store::queue(c, "email", to, Some(&msg.subject), &msg.text, at).await?;
        }
        Ok(Ok(emails.len()))
    })
    .await?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn form<'a>(kind: &'a str, emails: &'a str, url: &'a str, key: &'a str) -> CardForm<'a> {
        CardForm {
            kind,
            name: "On call",
            critical: true,
            warning: false,
            emails,
            url,
            key,
        }
    }

    #[test]
    fn a_card_needs_a_kind_a_name_a_level_and_what_its_kind_sends_to() {
        let err = |f: CardForm<'_>| check(&f).err().map(|e| e.to_string());
        assert_eq!(
            err(form("owner_email", "", "", "")),
            Some(AlertError::Fixed.to_string())
        );
        assert_eq!(
            err(form("sms", "", "", "")),
            Some(AlertError::BadFormat.to_string())
        );
        assert_eq!(
            err(CardForm {
                name: " ",
                ..form("email", "a@x.co", "", "")
            }),
            Some(AlertError::NoName.to_string())
        );
        assert_eq!(
            err(CardForm {
                critical: false,
                ..form("email", "a@x.co", "", "")
            }),
            Some(AlertError::NoLevel.to_string())
        );
        assert_eq!(
            err(form("email", " , ", "", "")),
            Some(AlertError::NoEmails.to_string())
        );
        assert_eq!(
            err(form("discord", "", "", "")),
            Some(AlertError::BadWebhook.to_string())
        );
        let ok = check(&form("email", "B@x.co, b@x.co\nc@x.co", "ignored", "")).unwrap();
        assert_eq!(
            (ok.emails, ok.url),
            (vec!["b@x.co".to_string(), "c@x.co".into()], None)
        );
        let hook = check(&form(
            "discord",
            "x@x.co",
            "https://discord.com/api/webhooks/1/x",
            "",
        ))
        .unwrap();
        assert_eq!(
            (hook.emails.len(), hook.url.as_deref()),
            (0, Some("https://discord.com/api/webhooks/1/x"))
        );
    }
}
