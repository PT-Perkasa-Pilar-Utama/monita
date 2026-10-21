//! SQL for users, invites, sessions.

use turso::Connection;

use crate::platform::caps;

use super::Role;

pub struct UserRow {
    pub id: i64,
    pub email: String,
    pub role: String,
    pub password_hash: Option<String>,
    pub disabled_at: Option<i64>,
    pub created_at: i64,
    /// Two-factor sign-in is on.
    pub two_factor: bool,
}

pub struct Invite {
    pub id: i64,
    pub user_id: i64,
    pub expires_at: i64,
    pub used_at: Option<i64>,
}

const USER_COLS: &str =
    "id, email, role, password_hash, disabled_at, created_at, totp_secret IS NOT NULL";

fn user_row(r: &turso::Row) -> turso::Result<UserRow> {
    Ok(UserRow {
        id: r.get(0)?,
        email: r.get(1)?,
        role: r.get(2)?,
        password_hash: r.get(3)?,
        disabled_at: r.get(4)?,
        created_at: r.get(5)?,
        two_factor: r.get::<i64>(6)? != 0,
    })
}

pub async fn insert_user(
    c: &Connection,
    email: &str,
    role: Role,
    password_hash: Option<&str>,
) -> anyhow::Result<Result<i64, super::AccountError>> {
    if find_user_by_email(c, email).await?.is_some() {
        return Ok(Err(super::AccountError::EmailTaken));
    }
    let users = crate::platform::db::query_i64(c, "SELECT count(*) FROM users", ()).await?;
    if users.unwrap_or(0) >= caps::USERS {
        return Ok(Err(super::AccountError::Full));
    }
    c.execute(
        "INSERT INTO users (email, role, password_hash) VALUES (?1, ?2, ?3)",
        (email, role.as_str(), password_hash),
    )
    .await?;
    Ok(Ok(c.last_insert_rowid()))
}

pub async fn find_user_by_email(c: &Connection, email: &str) -> anyhow::Result<Option<UserRow>> {
    let mut rows = c
        .query(
            format!("SELECT {USER_COLS} FROM users WHERE email = ?1"),
            [email],
        )
        .await?;
    Ok(match rows.next().await? {
        Some(r) => Some(user_row(&r)?),
        None => None,
    })
}

pub async fn active_tl_email(c: &Connection, user_id: i64) -> anyhow::Result<Option<String>> {
    let mut rows = c
        .query(
            "SELECT email FROM users WHERE id = ?1 AND role = 'tl' AND disabled_at IS NULL",
            [user_id],
        )
        .await?;
    Ok(match rows.next().await? {
        Some(r) => Some(r.get(0)?),
        None => None,
    })
}

pub async fn list_users(c: &Connection) -> anyhow::Result<Vec<UserRow>> {
    crate::platform::db::query_all(
        c,
        format!("SELECT {USER_COLS} FROM users ORDER BY role DESC, email LIMIT ?1"),
        [caps::USERS],
        user_row,
    )
    .await
}

pub async fn active_email(c: &Connection, user_id: i64) -> anyhow::Result<Option<String>> {
    let mut rows = c
        .query(
            "SELECT email FROM users WHERE id = ?1 AND disabled_at IS NULL",
            [user_id],
        )
        .await?;
    Ok(match rows.next().await? {
        Some(r) => Some(r.get(0)?),
        None => None,
    })
}

pub async fn set_role(c: &Connection, user_id: i64, role: Role, at: i64) -> anyhow::Result<bool> {
    let n = c
        .execute(
            "UPDATE users SET role = ?1, updated_at = ?2 WHERE id = ?3 AND disabled_at IS NULL",
            (role.as_str(), at, user_id),
        )
        .await?;
    Ok(n > 0)
}

/// Read inside the write that acts on another account: the request's role check ran before the
/// writer was taken, and another admin's change may have landed in between.
pub async fn is_active_super_admin(c: &Connection, user_id: i64) -> anyhow::Result<bool> {
    let n = crate::platform::db::query_i64(
        c,
        "SELECT count(*) FROM users WHERE id = ?1 AND role = 'super_admin' AND disabled_at IS NULL",
        [user_id],
    )
    .await?;
    Ok(n == Some(1))
}

pub async fn enable_user(c: &Connection, user_id: i64, at: i64) -> anyhow::Result<bool> {
    let n = c
        .execute(
            "UPDATE users SET disabled_at = NULL, updated_at = ?1 WHERE id = ?2 AND disabled_at IS NOT NULL",
            (at, user_id),
        )
        .await?;
    Ok(n > 0)
}

pub async fn disable_user(c: &Connection, user_id: i64, at: i64) -> anyhow::Result<bool> {
    let n = c
        .execute(
            "UPDATE users SET disabled_at = ?1, updated_at = ?1 WHERE id = ?2 AND disabled_at IS NULL",
            (at, user_id),
        )
        .await?;
    if n == 0 {
        return Ok(false);
    }
    c.execute("DELETE FROM sessions WHERE user_id = ?1", [user_id])
        .await?;
    c.execute("DELETE FROM invites WHERE user_id = ?1", [user_id])
        .await?;
    Ok(true)
}

/// Expired sessions and invites, and invites used more than 7 days ago.
/// Deletes up to `limit` of each; returns how many rows it deleted.
pub async fn purge_expired(c: &Connection, now: i64, limit: i64) -> anyhow::Result<u64> {
    let sessions = c
        .execute(
            "DELETE FROM sessions WHERE rowid IN (SELECT rowid FROM sessions WHERE expires_at < ?1 LIMIT ?2)",
            (now, limit),
        )
        .await?;
    let invites = c
        .execute(
            "DELETE FROM invites WHERE rowid IN (SELECT rowid FROM invites WHERE expires_at < ?1 OR (used_at IS NOT NULL AND used_at < ?2) LIMIT ?3)",
            (now, now - 7 * 86_400, limit),
        )
        .await?;
    Ok(sessions + invites)
}

/// Deletes the user's older invites and sessions, then stores the new invite.
pub async fn replace_invite(
    c: &Connection,
    user_id: i64,
    token_hash: &str,
    expires_at: i64,
) -> anyhow::Result<()> {
    c.execute("DELETE FROM invites WHERE user_id = ?1", [user_id])
        .await?;
    c.execute("DELETE FROM sessions WHERE user_id = ?1", [user_id])
        .await?;
    // The old password stops at once (US-04): a reset that clears two-factor must not leave a
    // leaked password working until the new link is used.
    c.execute(
        "UPDATE users SET password_hash = NULL WHERE id = ?1",
        [user_id],
    )
    .await?;
    c.execute(
        "INSERT INTO invites (user_id, token_hash, expires_at) VALUES (?1, ?2, ?3)",
        (user_id, token_hash, expires_at),
    )
    .await?;
    Ok(())
}

pub async fn find_invite(c: &Connection, token_hash: &str) -> anyhow::Result<Option<Invite>> {
    let mut rows = c
        .query(
            "SELECT i.id, i.user_id, i.expires_at, i.used_at FROM invites i JOIN users u ON u.id = i.user_id
             WHERE i.token_hash = ?1 AND u.disabled_at IS NULL",
            [token_hash],
        )
        .await?;
    Ok(match rows.next().await? {
        Some(r) => Some(Invite {
            id: r.get(0)?,
            user_id: r.get(1)?,
            expires_at: r.get(2)?,
            used_at: r.get(3)?,
        }),
        None => None,
    })
}

pub async fn use_invite(
    c: &Connection,
    invite_id: i64,
    user_id: i64,
    password_hash: &str,
    at: i64,
) -> anyhow::Result<()> {
    c.execute(
        "UPDATE invites SET used_at = ?1 WHERE id = ?2",
        (at, invite_id),
    )
    .await?;
    set_password(c, user_id, password_hash, at).await
}

/// Also signs the user out everywhere.
pub async fn set_password(
    c: &Connection,
    user_id: i64,
    password_hash: &str,
    at: i64,
) -> anyhow::Result<()> {
    c.execute(
        "UPDATE users SET password_hash = ?1, updated_at = ?2 WHERE id = ?3",
        (password_hash, at, user_id),
    )
    .await?;
    c.execute("DELETE FROM sessions WHERE user_id = ?1", [user_id])
        .await?;
    Ok(())
}

pub async fn insert_session(
    c: &Connection,
    user_id: i64,
    token_hash: &str,
    expires_at: i64,
) -> anyhow::Result<()> {
    c.execute(
        "INSERT INTO sessions (user_id, token_hash, expires_at) VALUES (?1, ?2, ?3)",
        (user_id, token_hash, expires_at),
    )
    .await?;
    Ok(())
}

pub async fn delete_session(c: &Connection, token_hash: &str) -> anyhow::Result<()> {
    c.execute("DELETE FROM sessions WHERE token_hash = ?1", [token_hash])
        .await?;
    Ok(())
}

pub async fn find_session_user(
    c: &Connection,
    token_hash: &str,
    at: i64,
) -> anyhow::Result<Option<UserRow>> {
    let mut rows = c
        .query(
            "SELECT u.id, u.email, u.role, u.password_hash, u.disabled_at, u.created_at, u.totp_secret IS NOT NULL
             FROM sessions s JOIN users u ON u.id = s.user_id
             WHERE s.token_hash = ?1 AND s.expires_at > ?2 AND u.disabled_at IS NULL",
            (token_hash, at),
        )
        .await?;
    Ok(match rows.next().await? {
        Some(r) => Some(user_row(&r)?),
        None => None,
    })
}
