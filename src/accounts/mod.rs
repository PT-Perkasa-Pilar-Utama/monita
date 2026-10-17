//! accounts module: users, invites, sessions.
//! Spec: docs/technical-specs/05-module-definitions.md section 5.1 and 09.

mod auth;
mod password;
pub mod routes;
mod store;

use crate::platform::{db::Db, now, page::PageError, token};
use password::{hash_password, verify_password};

pub use auth::{AdminUser, CurrentUser, clear_cookie, session_cookie, session_token};
pub use store::UserRow;

pub const SESSION_COOKIE: &str = "monita_session";
const SESSION_TTL_S: i64 = 14 * 86_400;
const INVITE_TTL_S: i64 = 72 * 3_600;
pub const MIN_PASSWORD: usize = 12;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    SuperAdmin,
    Tl,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Role::SuperAdmin => "super_admin",
            Role::Tl => "tl",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        match s {
            "super_admin" => Some(Role::SuperAdmin),
            "tl" => Some(Role::Tl),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct User {
    pub id: i64,
    pub email: String,
    pub role: Role,
}

impl User {
    pub fn is_admin(&self) -> bool {
        self.role == Role::SuperAdmin
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AccountError {
    #[error("An account with this email already exists.")]
    EmailTaken,
    #[error("Enter a valid email address.")]
    InvalidEmail,
    #[error("Use at least 12 characters.")]
    PasswordTooShort,
    #[error("Wrong email or password.")]
    BadCredentials,
    #[error("This link is invalid, expired or already used.")]
    InviteInvalid,
    #[error("No super admin has this email.")]
    NoSuchAdmin,
    #[error(
        "This Monita holds {} accounts, its limit.",
        crate::platform::caps::USERS
    )]
    Full,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// The form an email is stored and looked up in, or `None` for text no stored address can match
/// (over 254 bytes, no `@`), so junk costs no lookup. Sign-in uses only this, not the stricter
/// rule for new addresses, so no existing account is locked out.
fn email_key(raw: &str) -> Option<String> {
    let key = raw.trim().to_lowercase();
    (key.len() <= 254 && key.contains('@')).then_some(key)
}

/// A new address: one the SMTP server will take. ASCII only, because lettre refuses non-ASCII
/// addresses unless the server offers SMTPUTF8, and a name domain, not an `[ip]` literal.
pub fn normalize_email(raw: &str) -> Result<String, AccountError> {
    let email = email_key(raw).ok_or(AccountError::InvalidEmail)?;
    let valid = email.is_ascii()
        && !email.contains(char::is_whitespace)
        && matches!(email.split_once('@'), Some((l, d)) if !l.is_empty() && d.contains('.') && !d.contains('@') && !d.starts_with('['))
        && email.parse::<lettre::Address>().is_ok();
    if valid {
        Ok(email)
    } else {
        Err(AccountError::InvalidEmail)
    }
}

/// US-01: the first super admin, from the CLI.
pub async fn create_super_admin(
    db: &Db,
    email: &str,
    password: &str,
) -> Result<User, AccountError> {
    let email = normalize_email(email)?;
    let hash = hash_password(password).await?;
    let id = db
        .write(async |c| store::insert_user(c, &email, Role::SuperAdmin, Some(&hash)).await)
        .await??;
    Ok(User {
        id,
        email,
        role: Role::SuperAdmin,
    })
}

/// The CLI recovery path for a super admin who lost the password. Signs them out everywhere.
pub async fn reset_super_admin_password(
    db: &Db,
    email: &str,
    password: &str,
) -> Result<(), AccountError> {
    let email = email_key(email).ok_or(AccountError::NoSuchAdmin)?;
    let hash = hash_password(password).await?;
    db.write(
        async |c| match store::find_user_by_email(c, &email).await? {
            Some(u) if u.role == "super_admin" => {
                store::set_password(c, u.id, &hash, now()).await?;
                Ok(Ok(()))
            }
            _ => Ok(Err(AccountError::NoSuchAdmin)),
        },
    )
    .await?
}

/// Signs in. Returns the raw session token for the cookie.
pub async fn login(db: &Db, email: &str, password: &str) -> Result<String, AccountError> {
    let email = email_key(email).ok_or(AccountError::BadCredentials)?;
    let user = store::find_user_by_email(&db.read().await?, &email).await?;
    // ponytail: an unknown email skips argon2, so its timing differs; verify a dummy hash if enumeration matters.
    let Some(user) = user.filter(|u| u.disabled_at.is_none()) else {
        return Err(AccountError::BadCredentials);
    };
    let Some(stored) = user.password_hash.as_deref() else {
        return Err(AccountError::BadCredentials);
    };
    if !verify_password(password, stored).await {
        return Err(AccountError::BadCredentials);
    }
    new_session(db, user.id).await
}

async fn new_session(db: &Db, user_id: i64) -> Result<String, AccountError> {
    let raw = token::random_token();
    let hash = token::hash(&raw);
    db.write(async |c| store::insert_session(c, user_id, &hash, now() + SESSION_TTL_S).await)
        .await?;
    Ok(raw)
}

pub async fn logout(db: &Db, raw_token: &str) -> anyhow::Result<()> {
    let hash = token::hash(raw_token);
    db.write(async |c| store::delete_session(c, &hash).await)
        .await
}

/// US-02: creates a TL with no password and queues the invite email. Returns the invite link.
pub async fn invite_tl(db: &Db, base_url: &str, email: &str) -> Result<String, AccountError> {
    let email = normalize_email(email)?;
    let raw = token::random_token();
    let link = format!("{base_url}/invite/{raw}");
    let hash = token::hash(&raw);
    db.write(async |c| {
        let id = match store::insert_user(c, &email, Role::Tl, None).await? {
            Ok(id) => id,
            Err(e) => return Ok(Err(e)),
        };
        store::replace_invite(c, id, &hash, now() + INVITE_TTL_S).await?;
        queue_invite_email(c, &email, &link).await?;
        Ok(Ok(()))
    })
    .await??;
    Ok(link)
}

/// US-04: a new invite for an existing user. Older invites and all sessions stop working.
/// Returns the TL's email and the new link.
pub async fn reinvite(
    db: &Db,
    base_url: &str,
    user_id: i64,
) -> Result<(String, String), PageError> {
    let raw = token::random_token();
    let link = format!("{base_url}/invite/{raw}");
    let hash = token::hash(&raw);
    let email = db
        .write(async |c| {
            let Some(email) = store::active_tl_email(c, user_id).await? else {
                return Ok(None);
            };
            store::replace_invite(c, user_id, &hash, now() + INVITE_TTL_S).await?;
            queue_invite_email(c, &email, &link).await?;
            Ok(Some(email))
        })
        .await?
        .ok_or(PageError::NotFound)?;
    Ok((email, link))
}

async fn queue_invite_email(c: &turso::Connection, email: &str, link: &str) -> anyhow::Result<()> {
    let body = format!(
        "You are invited to Monita.\n\nSet your password here:\n{link}\n\nThe link works once and expires in 72 hours."
    );
    crate::notify::queue_email(c, email, "You are invited to Monita", &body, now()).await
}

pub async fn invite_is_usable(db: &Db, raw_token: &str) -> anyhow::Result<bool> {
    let invite = store::find_invite(&db.read().await?, &token::hash(raw_token)).await?;
    Ok(invite.is_some_and(|i| i.used_at.is_none() && i.expires_at > now()))
}

/// US-03 and US-04: sets the password, uses the invite up, signs the user in.
pub async fn accept_invite(
    db: &Db,
    raw_token: &str,
    password: &str,
) -> Result<String, AccountError> {
    if !invite_is_usable(db, raw_token).await? {
        return Err(AccountError::InviteInvalid);
    }
    let hash = hash_password(password).await?;
    let token_hash = token::hash(raw_token);
    // The write re-checks the invite: it may be used or revoked while argon2 runs.
    let user_id = db
        .write(async |c| {
            let Some(invite) = store::find_invite(c, &token_hash).await? else {
                return Ok(None);
            };
            if invite.used_at.is_some() || invite.expires_at <= now() {
                return Ok(None);
            }
            store::use_invite(c, invite.id, invite.user_id, &hash, now()).await?;
            Ok(Some(invite.user_id))
        })
        .await?
        .ok_or(AccountError::InviteInvalid)?;
    new_session(db, user_id).await
}

pub async fn list_users(db: &Db) -> anyhow::Result<Vec<UserRow>> {
    store::list_users(&db.read().await?).await
}

pub async fn is_active_tl(c: &turso::Connection, user_id: i64) -> anyhow::Result<bool> {
    Ok(store::active_tl_email(c, user_id).await?.is_some())
}

/// Retention (US-32): runs inside the caller's write.
/// Deletes up to `limit` sessions and `limit` invites; returns how many. The caller loops.
pub async fn purge_expired(c: &turso::Connection, now: i64, limit: i64) -> anyhow::Result<u64> {
    store::purge_expired(c, now, limit).await
}

/// Disabling is final: the user's invites go too, and only an active TL can be disabled.
pub async fn disable(db: &Db, user_id: i64) -> Result<(), PageError> {
    if db
        .write(async |c| store::disable_tl(c, user_id, now()).await)
        .await?
    {
        Ok(())
    } else {
        Err(PageError::NotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderMap, header};

    #[test]
    fn email_is_trimmed_lowercased_and_checked() {
        assert_eq!(
            normalize_email("  Rina@Company.co.id ").unwrap(),
            "rina@company.co.id"
        );
        assert!(normalize_email("rina").is_err());
        assert!(normalize_email("@company.co.id").is_err());
        assert!(normalize_email("ri na@company.co.id").is_err());
        for bad in [
            "oncall@company.co.id.",
            "a@b..c",
            "a,b@x.co",
            "a@-x.co",
            "jörg@company.co.id",
            "rina@münchen.de",
            "a@[1.2.3.4]",
        ] {
            assert!(
                normalize_email(bad).is_err(),
                "{bad}: the mailer would refuse it"
            );
        }
    }

    #[test]
    fn a_lookup_key_is_lenient_but_junk_costs_nothing() {
        assert_eq!(email_key(" Jörg@X.co ").as_deref(), Some("jörg@x.co"));
        assert!(email_key(&"a".repeat(300)).is_none());
        assert!(email_key("no-at-sign").is_none());
    }

    #[test]
    fn reads_the_session_cookie_among_others() {
        let mut h = HeaderMap::new();
        h.insert(
            header::COOKIE,
            "a=1; monita_session=abc; b=2".parse().unwrap(),
        );
        assert_eq!(session_token(&h).as_deref(), Some("abc"));
        h.insert(header::COOKIE, "monita_session=".parse().unwrap());
        assert_eq!(session_token(&h), None);
    }
}
