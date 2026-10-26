//! accounts module: users, invites, sessions.
//! Spec: docs/technical-specs/05-module-definitions.md section 5.1 and 09.

mod admin;
pub mod admin_api;
pub mod api;
mod auth;
mod password;
mod settings;
mod store;
pub mod totp;
mod two_factor;
mod user;

use crate::platform::{db::Db, now, page::PageError, token};
use password::{hash_password, verify_password};

pub use admin::{AdminError, disable, enable, reset_sign_in, set_role, turn_off_two_factor};
pub use auth::{AccountUser, AdminUser, CurrentUser, clear_cookie, session_cookie, session_token};
pub use store::UserRow;
pub use two_factor::TwoFactorError;
pub use user::{KeyGrant, Role, User};

pub const SESSION_COOKIE: &str = "monita_session";
const SESSION_TTL_S: i64 = 14 * 86_400;
const INVITE_TTL_S: i64 = 72 * 3_600;
pub const MIN_PASSWORD: usize = 8;

#[derive(Debug, thiserror::Error)]
pub enum AccountError {
    #[error("An account with this email already exists.")]
    EmailTaken,
    #[error("Enter a valid email address.")]
    InvalidEmail,
    #[error("Use at least 8 characters.")]
    PasswordTooShort,
    #[error("Wrong email or password.")]
    BadCredentials,
    #[error("Too many wrong passwords for this account, so wait {0} seconds and try again.")]
    TooMany(i64),
    #[error("This link is invalid, expired or already used.")]
    InviteInvalid,
    #[error("No super admin has this email.")]
    NoSuchAdmin,
    #[error("No active user has this email.")]
    NoSuchUser,
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
        key: None,
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
                // Q19: the CLI is also the way back in for a super admin who lost the phone.
                two_factor::clear(c, u.id).await?;
                Ok(Ok(()))
            }
            _ => Ok(Err(AccountError::NoSuchAdmin)),
        },
    )
    .await?
}

/// What a right password leads to.
pub enum SignIn {
    /// The raw session token for the cookie.
    Session(String),
    /// Two-factor is on: the raw challenge token for the code step.
    Code(String),
}

/// Signs in, or asks for the code when two-factor is on.
/// Wrong passwords one account may take in `WRONG_WINDOW_S` before sign-in refuses it for the
/// rest of the window. Counted per account, never per address, so one office or one carrier's
/// shared address can never lock everyone out (v1.1 Q45).
const WRONG_PASSWORDS: u32 = 10;
const WRONG_WINDOW_S: i64 = 900;

pub async fn login(
    db: &Db,
    limiter: &crate::platform::rate_limit::Limiter,
    email: &str,
    password: &str,
) -> Result<SignIn, AccountError> {
    use crate::platform::rate_limit::Verdict;
    let email = email_key(email).ok_or(AccountError::BadCredentials)?;
    // An unknown email has a budget too, so the answer never tells whether an account exists.
    let budget = format!("signin:{email}");
    if let Verdict::Deny(retry) = limiter.over(&budget, WRONG_PASSWORDS, WRONG_WINDOW_S, now()) {
        return Err(AccountError::TooMany(retry));
    }
    let user = store::find_user_by_email(&db.read().await?, &email).await?;
    // An unknown, disabled or not yet activated account costs one argon2 run, as a real one does.
    let signed_in = match user.as_ref().filter(|u| u.disabled_at.is_none()) {
        Some(u) => match u.password_hash.as_deref() {
            Some(stored) => verify_password(password, stored).await,
            None => password::refuse_like_a_check(password).await,
        },
        None => password::refuse_like_a_check(password).await,
    };
    match user {
        Some(u) if signed_in => {
            if two_factor::is_on(&db.read().await?, u.id).await? {
                Ok(SignIn::Code(two_factor::challenge(db, u.id, now()).await?))
            } else {
                Ok(SignIn::Session(new_session(db, u.id).await?))
            }
        }
        _ => {
            limiter.hit(&budget, WRONG_PASSWORDS, WRONG_WINDOW_S, now());
            Err(AccountError::BadCredentials)
        }
    }
}

/// The code step of a two-factor sign-in. Returns the raw session token.
pub async fn login_code(
    db: &Db,
    limiter: &crate::platform::rate_limit::Limiter,
    challenge: &str,
    code: &str,
) -> Result<String, TwoFactorError> {
    let user_id = two_factor::finish(db, limiter, challenge, code, now()).await?;
    new_session(db, user_id)
        .await
        .map_err(|e| TwoFactorError::Other(e.into()))
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
    invite(db, base_url, email, Role::Tl).await
}

/// Creates a user of `role` with no password and queues the invite email. Returns the link.
pub async fn invite(
    db: &Db,
    base_url: &str,
    email: &str,
    role: Role,
) -> Result<String, AccountError> {
    let email = normalize_email(email)?;
    let raw = token::random_token();
    let link = format!("{base_url}/invite/{raw}");
    let hash = token::hash(&raw);
    db.write(async |c| {
        let id = match store::insert_user(c, &email, role, None).await? {
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

/// US-04: a new invite for an existing user, which resets their sign-in. Older invites and
/// all sessions stop working. Returns the user's email and the new link.
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
            let Some(email) = store::active_email(c, user_id).await? else {
                return Ok(None);
            };
            store::replace_invite(c, user_id, &hash, now() + INVITE_TTL_S).await?;
            // A reset sign-in starts over: the user sets a password and may turn two-factor on.
            two_factor::clear(c, user_id).await?;
            queue_invite_email(c, &email, &link).await?;
            Ok(Some(email))
        })
        .await?
        .ok_or(PageError::NotFound)?;
    Ok((email, link))
}

/// The fail-safe for a user who lost both the authenticator and the recovery codes
/// (`monita admin reset-sign-in`): Reset sign-in by email, for any active user. Returns the link
/// the email carries, for the operator to pass on when mail does not work.
pub async fn reset_sign_in_by_email(
    db: &Db,
    base_url: &str,
    email: &str,
) -> Result<String, AccountError> {
    let email = email_key(email).ok_or(AccountError::NoSuchUser)?;
    let user = match store::find_user_by_email(&db.read().await?, &email).await? {
        Some(u) if u.disabled_at.is_none() => u,
        _ => return Err(AccountError::NoSuchUser),
    };
    match reinvite(db, base_url, user.id).await {
        Ok((_, link)) => Ok(link),
        Err(PageError::Internal(e)) => Err(e.into()),
        Err(_) => Err(AccountError::NoSuchUser),
    }
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

/// Does an active user sign in with this address? Emails are stored lowercase.
pub async fn email_is_active_user(c: &turso::Connection, email: &str) -> anyhow::Result<bool> {
    store::email_is_active_user(c, &email.trim().to_lowercase()).await
}

/// The email of a user who is not disabled.
pub async fn active_email(c: &turso::Connection, user_id: i64) -> anyhow::Result<Option<String>> {
    store::active_email(c, user_id).await
}

pub async fn is_active_tl(c: &turso::Connection, user_id: i64) -> anyhow::Result<bool> {
    Ok(store::active_tl_email(c, user_id).await?.is_some())
}

/// May own a project now: an active tech lead or an active super admin, never a supervisor.
/// Read on the caller's write, so a role changed since the request began counts.
pub async fn can_own_projects(c: &turso::Connection, user_id: i64) -> anyhow::Result<bool> {
    Ok(store::active_tl_email(c, user_id).await?.is_some()
        || store::is_active_super_admin(c, user_id).await?)
}

/// Retention (US-32): runs inside the caller's write.
/// Deletes up to `limit` each of sessions, invites and sign-in challenges; returns how many. The
/// caller loops.
pub async fn purge_expired(c: &turso::Connection, now: i64, limit: i64) -> anyhow::Result<u64> {
    Ok(store::purge_expired(c, now, limit).await? + two_factor::purge(c, now, limit).await?)
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
