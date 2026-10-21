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

pub async fn set_avatar(
    c: &Connection,
    user_id: i64,
    mime: &str,
    data: Vec<u8>,
    at: i64,
) -> anyhow::Result<()> {
    c.execute(
        "INSERT INTO user_avatars (user_id, mime, data) VALUES (?1, ?2, ?3)
         ON CONFLICT (user_id) DO UPDATE SET mime = excluded.mime, data = excluded.data",
        (user_id, mime, turso::Value::from(data)),
    )
    .await?;
    c.execute(
        "UPDATE users SET avatar_at = ?1, updated_at = ?1 WHERE id = ?2",
        (at, user_id),
    )
    .await?;
    Ok(())
}

pub async fn remove_avatar(c: &Connection, user_id: i64, at: i64) -> anyhow::Result<()> {
    c.execute("DELETE FROM user_avatars WHERE user_id = ?1", [user_id])
        .await?;
    c.execute(
        "UPDATE users SET avatar_at = NULL, updated_at = ?1 WHERE id = ?2",
        (at, user_id),
    )
    .await?;
    Ok(())
}

/// The picture's type and bytes.
pub async fn avatar(c: &Connection, user_id: i64) -> anyhow::Result<Option<(String, Vec<u8>)>> {
    Ok(crate::platform::db::query_all(
        c,
        "SELECT mime, data FROM user_avatars WHERE user_id = ?1",
        [user_id],
        |r| {
            let data = r.get_value(1)?.as_blob().cloned().unwrap_or_default();
            Ok((r.get(0)?, data))
        },
    )
    .await?
    .pop())
}

/// When the picture last changed, which versions its URL; `None` without one.
pub async fn avatar_at(c: &Connection, user_id: i64) -> anyhow::Result<Option<i64>> {
    let mut rows = c
        .query("SELECT avatar_at FROM users WHERE id = ?1", [user_id])
        .await?;
    Ok(match rows.next().await? {
        Some(r) => r.get::<Option<i64>>(0)?,
        None => None,
    })
}
