//! SQL for a user's own settings.

use turso::Connection;

pub async fn password_hash(c: &Connection, user_id: i64) -> anyhow::Result<Option<String>> {
    let mut rows = c
        .query(
            "SELECT password_hash FROM users WHERE id = ?1 AND disabled_at IS NULL",
            [user_id],
        )
        .await?;
    Ok(match rows.next().await? {
        Some(r) => r.get::<Option<String>>(0)?,
        None => None,
    })
}

/// Swaps the hash only if it is still `old`, and ends every other session of the user. False when
/// the password changed in between.
pub async fn replace_password(
    c: &Connection,
    user_id: i64,
    old: &str,
    new: &str,
    keep_session: &str,
    at: i64,
) -> anyhow::Result<bool> {
    let n = c
        .execute(
            "UPDATE users SET password_hash = ?1, updated_at = ?2 WHERE id = ?3 AND password_hash = ?4",
            (new, at, user_id, old),
        )
        .await?;
    if n == 0 {
        return Ok(false);
    }
    c.execute(
        "DELETE FROM sessions WHERE user_id = ?1 AND token_hash <> ?2",
        (user_id, keep_session),
    )
    .await?;
    Ok(true)
}
