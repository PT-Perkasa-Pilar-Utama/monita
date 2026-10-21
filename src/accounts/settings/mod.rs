//! The signed-in user's own settings: their password. Any role may change its own, a supervisor
//! too (`AccountUser`). Contract: docs/api-specs/02-authentication.md section 2.5.

pub mod api;
mod store;

use super::{AccountError, MIN_PASSWORD, password};
use crate::platform::{
    db::Db,
    rate_limit::{Limiter, Verdict},
    token,
};

#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("Enter your current password.")]
    NoCurrent,
    #[error("This is not your current password.")]
    WrongPassword,
    #[error("Use at least 12 characters.")]
    TooShort,
    #[error("Too many tries. Wait {0} seconds, then try again.")]
    TooMany(i64),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Password changes one account may try in `TRIES_WINDOW_S`, right or wrong. A stolen session
/// must not be a way to guess the password, which the user may use elsewhere too.
const TRIES: u32 = 10;
const TRIES_WINDOW_S: i64 = 900;

/// Checks `current`, stores `new`, and signs the user out everywhere but this session.
pub async fn change_password(
    db: &Db,
    limiter: &Limiter,
    user_id: i64,
    session_raw: &str,
    current: &str,
    new: &str,
    at: i64,
) -> Result<(), SettingsError> {
    if current.is_empty() {
        return Err(SettingsError::NoCurrent);
    }
    // Counted before the check: requests sent at once cannot all pass a check made before any
    // of them counts.
    if let Verdict::Deny(retry) =
        limiter.hit(&format!("password:{user_id}"), TRIES, TRIES_WINDOW_S, at)
    {
        return Err(SettingsError::TooMany(retry));
    }
    let stored = store::password_hash(&db.read().await?, user_id).await?;
    let Some(stored) = stored else {
        return Err(SettingsError::WrongPassword);
    };
    if !password::verify_password(current, &stored).await {
        return Err(SettingsError::WrongPassword);
    }
    if new.chars().count() < MIN_PASSWORD {
        return Err(SettingsError::TooShort);
    }
    let hash = password::hash_password(new).await.map_err(|e| match e {
        AccountError::PasswordTooShort => SettingsError::TooShort,
        e => SettingsError::Other(e.into()),
    })?;
    let keep = token::hash(session_raw);
    let changed = db
        .write(async |c| store::replace_password(c, user_id, &stored, &hash, &keep, at).await)
        .await?;
    // A change from another session landed between the check and this write.
    if !changed {
        return Err(SettingsError::WrongPassword);
    }
    Ok(())
}
