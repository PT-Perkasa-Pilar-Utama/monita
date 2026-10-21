//! SQL for the two-factor columns of users, recovery_codes and sign_in_challenges.

use turso::Connection;

use crate::platform::db::{query_all, query_i64};

/// (confirmed secret, pending secret, last used step) of a user.
pub async fn secrets(
    c: &Connection,
    user_id: i64,
) -> anyhow::Result<Option<(Option<String>, Option<String>, Option<i64>)>> {
    Ok(query_all(
        c,
        "SELECT totp_secret, totp_pending, totp_step FROM users WHERE id = ?1",
        [user_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )
    .await?
    .into_iter()
    .next())
}

pub async fn set_pending(
    c: &Connection,
    user_id: i64,
    secret: &str,
    expires_at: i64,
) -> anyhow::Result<()> {
    c.execute(
        "UPDATE users SET totp_pending = ?2, totp_pending_expires_at = ?3 WHERE id = ?1",
        (user_id, secret, expires_at),
    )
    .await?;
    Ok(())
}

/// The waiting secret, unless it expired.
pub async fn pending(c: &Connection, user_id: i64, now: i64) -> anyhow::Result<Option<String>> {
    Ok(query_all(
        c,
        "SELECT totp_pending FROM users WHERE id = ?1 AND totp_pending_expires_at > ?2",
        (user_id, now),
        |r| r.get::<Option<String>>(0),
    )
    .await?
    .into_iter()
    .next()
    .flatten())
}

/// The pending secret becomes the secret; `code_hashes` replace any older recovery codes.
pub async fn enable(
    c: &Connection,
    user_id: i64,
    step: i64,
    code_hashes: &[String],
) -> anyhow::Result<()> {
    c.execute(
        "UPDATE users SET totp_secret = totp_pending, totp_pending = NULL, totp_pending_expires_at = NULL, totp_step = ?2 WHERE id = ?1",
        (user_id, step),
    )
    .await?;
    c.execute("DELETE FROM recovery_codes WHERE user_id = ?1", [user_id])
        .await?;
    for h in code_hashes {
        c.execute(
            "INSERT INTO recovery_codes (user_id, code_hash) VALUES (?1, ?2)",
            (user_id, h.as_str()),
        )
        .await?;
    }
    Ok(())
}

pub async fn set_step(c: &Connection, user_id: i64, step: i64) -> anyhow::Result<()> {
    c.execute(
        "UPDATE users SET totp_step = ?2 WHERE id = ?1",
        (user_id, step),
    )
    .await?;
    Ok(())
}

/// Uses an unused recovery code of the user; false when none matches.
pub async fn use_recovery(
    c: &Connection,
    user_id: i64,
    code_hash: &str,
    at: i64,
) -> anyhow::Result<bool> {
    let n = c
        .execute(
            "UPDATE recovery_codes SET used_at = ?3 WHERE user_id = ?1 AND code_hash = ?2 AND used_at IS NULL",
            (user_id, code_hash, at),
        )
        .await?;
    Ok(n == 1)
}

pub async fn codes_left(c: &Connection, user_id: i64) -> anyhow::Result<i64> {
    Ok(query_i64(
        c,
        "SELECT count(*) FROM recovery_codes WHERE user_id = ?1 AND used_at IS NULL",
        [user_id],
    )
    .await?
    .unwrap_or(0))
}

/// Two-factor off: no secret, no pending secret, no codes, no open challenge.
pub async fn clear(c: &Connection, user_id: i64) -> anyhow::Result<()> {
    c.execute(
        "UPDATE users SET totp_secret = NULL, totp_pending = NULL, totp_pending_expires_at = NULL, totp_step = NULL WHERE id = ?1",
        [user_id],
    )
    .await?;
    c.execute("DELETE FROM recovery_codes WHERE user_id = ?1", [user_id])
        .await?;
    c.execute(
        "DELETE FROM sign_in_challenges WHERE user_id = ?1",
        [user_id],
    )
    .await?;
    Ok(())
}

pub async fn insert_challenge(
    c: &Connection,
    token_hash: &str,
    user_id: i64,
    expires_at: i64,
) -> anyhow::Result<()> {
    c.execute(
        "INSERT INTO sign_in_challenges (token_hash, user_id, expires_at) VALUES (?1, ?2, ?3)",
        (token_hash, user_id, expires_at),
    )
    .await?;
    Ok(())
}

/// (user_id, wrong attempts) of a live challenge whose user is not disabled.
pub async fn find_challenge(
    c: &Connection,
    token_hash: &str,
    at: i64,
) -> anyhow::Result<Option<(i64, i64)>> {
    Ok(query_all(
        c,
        "SELECT ch.user_id, ch.attempts FROM sign_in_challenges ch JOIN users u ON u.id = ch.user_id
         WHERE ch.token_hash = ?1 AND ch.expires_at > ?2 AND u.disabled_at IS NULL",
        (token_hash, at),
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .await?
    .into_iter()
    .next())
}

pub async fn count_attempt(c: &Connection, token_hash: &str) -> anyhow::Result<()> {
    c.execute(
        "UPDATE sign_in_challenges SET attempts = attempts + 1 WHERE token_hash = ?1",
        [token_hash],
    )
    .await?;
    Ok(())
}

pub async fn delete_challenge(c: &Connection, token_hash: &str) -> anyhow::Result<()> {
    c.execute(
        "DELETE FROM sign_in_challenges WHERE token_hash = ?1",
        [token_hash],
    )
    .await?;
    Ok(())
}

/// Up to `limit` expired challenges; returns how many it deleted.
/// Expired sign-in challenges, and waiting secrets nobody confirmed in time; up to `limit` of each.
pub async fn purge(c: &Connection, now: i64, limit: i64) -> anyhow::Result<u64> {
    let challenges = c
        .execute(
            "DELETE FROM sign_in_challenges WHERE rowid IN (SELECT rowid FROM sign_in_challenges WHERE expires_at < ?1 LIMIT ?2)",
            (now, limit),
        )
        .await?;
    let pending = c
        .execute(
            "UPDATE users SET totp_pending = NULL, totp_pending_expires_at = NULL
             WHERE id IN (SELECT id FROM users WHERE totp_pending_expires_at <= ?1 LIMIT ?2)",
            (now, limit),
        )
        .await?;
    Ok(challenges + pending)
}
