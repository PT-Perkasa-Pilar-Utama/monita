//! Two-factor sign-in (v1.1 decision Q19, issue #12).
//!
//! A user's two-factor state:
//!
//! | From    | To      | Actor and guard                                                        |
//! |---------|---------|------------------------------------------------------------------------|
//! | off     | pending | the user: `start` shows a new secret                                   |
//! | pending | pending | the user: `start` again replaces the secret                            |
//! | pending | on      | the user: `confirm` with a right code; 10 recovery codes, shown once   |
//! | on      | off     | the user with a right code or recovery code; a super admin for another |
//! |         |         | user; `monita admin reset-password`; Reset sign-in (a new invite)      |
//!
//! A sign-in challenge: issued after a right password, then used by a right code, or ended by
//! five wrong codes or five minutes. Used and ended are terminal.

pub mod api;
mod store;

use turso::Connection;

use super::totp;
use crate::platform::{
    db::Db,
    rate_limit::{Limiter, Verdict},
    token,
};

pub use store::{clear, purge};

const CHALLENGE_TTL_S: i64 = 300;
/// How long a key shown at setup waits for its first code (v1.1 Q32).
const PENDING_TTL_S: i64 = 600;
const MAX_ATTEMPTS: i64 = 5;
const RECOVERY_CODES: usize = 10;
const ISSUER: &str = "Monita";

#[derive(Debug, thiserror::Error)]
pub enum TwoFactorError {
    #[error("Wrong code. Use the code your authenticator app shows now, or a recovery code.")]
    WrongCode,
    #[error("Two-factor sign-in is already on.")]
    AlreadyOn,
    #[error("Two-factor sign-in is off.")]
    NotOn,
    #[error("This key expired or was replaced. Select Turn on to get a new one.")]
    NotPending,
    #[error("This sign-in has expired. Sign in again.")]
    ChallengeGone,
    /// Seconds until the account's wrong-code budget resets.
    #[error("Too many wrong codes for this account. Wait {} minutes and try again.", (.0 + 59) / 60)]
    TooMany(i64),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// What the account page shows.
#[derive(serde::Serialize)]
pub struct Status {
    pub on: bool,
    pub codes_left: i64,
}

pub async fn status(db: &Db, user_id: i64) -> anyhow::Result<Status> {
    let c = db.read().await?;
    let on = matches!(store::secrets(&c, user_id).await?, Some((Some(_), _, _)));
    Ok(Status {
        on,
        codes_left: store::codes_left(&c, user_id).await?,
    })
}

/// What the setup step shows: the secret to type, its `otpauth://` link, and that link as a QR code.
#[derive(serde::Serialize)]
pub struct Setup {
    pub secret: String,
    pub link: String,
    /// An SVG document, dark modules on white, for an `<img>`.
    pub qr: String,
}

/// The link as a QR code. A phone camera reads dark on light only, so the colours are fixed.
fn qr_svg(link: &str) -> anyhow::Result<String> {
    use qrcode::{QrCode, render::svg};
    let code = QrCode::new(link.as_bytes())?;
    Ok(code
        .render::<svg::Color<'_>>()
        // 4 px a module: whole pixels at the size the page shows it.
        .module_dimensions(4, 4)
        .dark_color(svg::Color("#000000"))
        .light_color(svg::Color("#ffffff"))
        .build())
}

/// A new secret and its `otpauth://` link, waiting for `confirm`.
pub async fn start(db: &Db, user_id: i64, email: &str, now: i64) -> Result<Setup, TwoFactorError> {
    let secret = totp::new_secret();
    db.write(async |c| {
        if matches!(store::secrets(c, user_id).await?, Some((Some(_), _, _))) {
            return Ok(Err(TwoFactorError::AlreadyOn));
        }
        store::set_pending(c, user_id, &secret, now + PENDING_TTL_S).await?;
        Ok(Ok(()))
    })
    .await??;
    let link = totp::link(ISSUER, email, &secret);
    let qr = qr_svg(&link)?;
    Ok(Setup { secret, link, qr })
}

/// A recovery code as typed, without case, spaces or dashes; None when it cannot be one.
fn recovery_key(code: &str) -> Option<String> {
    let key: String = code
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .collect();
    (key.len() == 10).then_some(key)
}

/// `abcde-fghij`: 50 random bits. Five wrong tries end a sign-in, so that is plenty.
fn new_recovery_code() -> String {
    let raw = token::random_token();
    format!("{}-{}", &raw[..5], &raw[5..10])
}

/// Turns two-factor on with the first right code. Returns the recovery codes, shown once.
pub async fn confirm(
    db: &Db,
    user_id: i64,
    code: &str,
    now: i64,
) -> Result<Vec<String>, TwoFactorError> {
    let codes: Vec<String> = (0..RECOVERY_CODES).map(|_| new_recovery_code()).collect();
    let hashes: Vec<String> = codes
        .iter()
        .map(|c| token::hash(&recovery_key(c).unwrap_or_default()))
        .collect();
    db.write(async |c| {
        if matches!(store::secrets(c, user_id).await?, Some((Some(_), _, _))) {
            return Ok(Err(TwoFactorError::AlreadyOn));
        }
        let Some(pending) = store::pending(c, user_id, now).await? else {
            return Ok(Err(TwoFactorError::NotPending));
        };
        let Some(step) = totp::check(&pending, code, now, None) else {
            return Ok(Err(TwoFactorError::WrongCode));
        };
        store::enable(c, user_id, step, &hashes).await?;
        Ok(Ok(()))
    })
    .await??;
    Ok(codes)
}

/// A right time code (used once) or an unused recovery code (used up), on the caller's write.
async fn spend_code(
    c: &Connection,
    user_id: i64,
    code: &str,
    now: i64,
) -> Result<bool, TwoFactorError> {
    let Some((Some(secret), _, last)) = store::secrets(c, user_id).await? else {
        return Err(TwoFactorError::NotOn);
    };
    if let Some(step) = totp::check(&secret, code, now, last) {
        store::set_step(c, user_id, step).await?;
        return Ok(true);
    }
    match recovery_key(code) {
        Some(key) => Ok(store::use_recovery(c, user_id, &token::hash(&key), now).await?),
        None => Ok(false),
    }
}

/// Wrong codes one account may give in `WRONG_WINDOW_S`, across sign-in challenges and the
/// account page. A right password makes a new challenge each time, so the per-challenge cap
/// alone does not bound the guesses: each guess has 3 right answers in a million (one step of
/// drift either way).
const WRONG_CODES: u32 = 10;
const WRONG_WINDOW_S: i64 = 900;

fn budget_key(user_id: i64) -> String {
    format!("2fa:{user_id}")
}

/// Refuses even a right code while the account's wrong-code budget is spent.
fn spent(limiter: &Limiter, user_id: i64, now: i64) -> Option<TwoFactorError> {
    match limiter.over(&budget_key(user_id), WRONG_CODES, WRONG_WINDOW_S, now) {
        Verdict::Deny(retry) => Some(TwoFactorError::TooMany(retry)),
        Verdict::Allow => None,
    }
}

fn count_wrong(limiter: &Limiter, user_id: i64, now: i64) {
    limiter.hit(&budget_key(user_id), WRONG_CODES, WRONG_WINDOW_S, now);
}

/// The user turns two-factor off, with a code.
pub async fn turn_off(
    db: &Db,
    limiter: &Limiter,
    user_id: i64,
    code: &str,
    now: i64,
) -> Result<(), TwoFactorError> {
    // Checked inside the single writer, so requests sent at once cannot all pass it first.
    db.write(async |c| {
        if let Some(e) = spent(limiter, user_id, now) {
            return Ok(Err(e));
        }
        match spend_code(c, user_id, code, now).await {
            Ok(true) => {
                store::clear(c, user_id).await?;
                Ok(Ok(()))
            }
            Ok(false) => {
                count_wrong(limiter, user_id, now);
                Ok(Err(TwoFactorError::WrongCode))
            }
            // `?` would turn NotOn into a 500.
            Err(e) => Ok(Err(e)),
        }
    })
    .await?
}

/// Whether the user must give a code after the password.
pub async fn is_on(c: &Connection, user_id: i64) -> anyhow::Result<bool> {
    Ok(matches!(
        store::secrets(c, user_id).await?,
        Some((Some(_), _, _))
    ))
}

/// After a right password: a challenge for the code step. Returns the raw token, sent once.
pub async fn challenge(db: &Db, user_id: i64, now: i64) -> anyhow::Result<String> {
    let raw = token::random_token();
    let hash = token::hash(&raw);
    db.write(async |c| store::insert_challenge(c, &hash, user_id, now + CHALLENGE_TTL_S).await)
        .await?;
    Ok(raw)
}

/// The code step. A right code uses the challenge up and returns the user; a wrong one counts,
/// and the fifth ends the challenge.
pub async fn finish(
    db: &Db,
    limiter: &Limiter,
    raw_challenge: &str,
    code: &str,
    now: i64,
) -> Result<i64, TwoFactorError> {
    let hash = token::hash(raw_challenge);
    db.write(async |c| {
        let Some((user_id, attempts)) = store::find_challenge(c, &hash, now).await? else {
            return Ok(Err(TwoFactorError::ChallengeGone));
        };
        if let Some(e) = spent(limiter, user_id, now) {
            return Ok(Err(e));
        }
        match spend_code(c, user_id, code, now).await {
            Ok(true) => {
                store::delete_challenge(c, &hash).await?;
                Ok(Ok(user_id))
            }
            Ok(false) if attempts + 1 >= MAX_ATTEMPTS => {
                count_wrong(limiter, user_id, now);
                store::delete_challenge(c, &hash).await?;
                Ok(Err(TwoFactorError::ChallengeGone))
            }
            Ok(false) => {
                count_wrong(limiter, user_id, now);
                store::count_attempt(c, &hash).await?;
                Ok(Err(TwoFactorError::WrongCode))
            }
            // Turned off since the password step: sign in again with the password alone.
            Err(TwoFactorError::NotOn) => {
                store::delete_challenge(c, &hash).await?;
                Ok(Err(TwoFactorError::ChallengeGone))
            }
            Err(e) => Ok(Err(e)),
        }
    })
    .await?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_recovery_code_reads_without_case_spaces_or_dashes() {
        assert_eq!(recovery_key("ABCDE-fghij").as_deref(), Some("abcdefghij"));
        assert_eq!(recovery_key(" abcde fghij ").as_deref(), Some("abcdefghij"));
        assert_eq!(recovery_key("123456"), None, "a time code is not one");
        let code = new_recovery_code();
        assert_eq!(code.len(), 11);
        assert!(recovery_key(&code).is_some());
    }
}
