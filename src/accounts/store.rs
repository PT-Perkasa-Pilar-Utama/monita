//! SQL for users, invites, sessions.

use turso::Connection;

use super::Role;

pub struct UserRow {
    pub id: i64,
    pub email: String,
    pub role: String,
    pub password_hash: Option<String>,
    pub disabled_at: Option<i64>,
    pub created_at: i64,
}

pub struct Invite {
    pub id: i64,
    pub user_id: i64,
    pub expires_at: i64,
    pub used_at: Option<i64>,
}

const USER_COLS: &str = "id, email, role, password_hash, disabled_at, created_at";

fn user_row(r: &turso::Row) -> turso::Result<UserRow> {
    Ok(UserRow {
        id: r.get(0)?,
        email: r.get(1)?,
        role: r.get(2)?,
        password_hash: r.get(3)?,
        disabled_at: r.get(4)?,
        created_at: r.get(5)?,
    })
}

pub async fn insert_user(
    c: &Connection,
    email: &str,
    role: Role,
    password_hash: Option<&str>,
) -> anyhow::Result<Option<i64>> {
    if find_user_by_email(c, email).await?.is_some() {
        return Ok(None);
    }
    c.execute(
        "INSERT INTO users (email, role, password_hash) VALUES (?1, ?2, ?3)",
        (email, role.as_str(), password_hash),
    )
    .await?;
    Ok(Some(c.last_insert_rowid()))
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
    let mut rows = c
        .query(
            format!("SELECT {USER_COLS} FROM users ORDER BY role DESC, email"),
            (),
        )
        .await?;
    let mut out = Vec::new();
    while let Some(r) = rows.next().await? {
        out.push(user_row(&r)?);
    }
    Ok(out)
}

pub async fn disable_tl(c: &Connection, user_id: i64, at: i64) -> anyhow::Result<bool> {
    let n = c
        .execute(
            "UPDATE users SET disabled_at = ?1, updated_at = ?1 WHERE id = ?2 AND role = 'tl' AND disabled_at IS NULL",
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
pub async fn purge_expired(c: &Connection, now: i64) -> anyhow::Result<()> {
    c.execute("DELETE FROM sessions WHERE expires_at < ?1", [now])
        .await?;
    c.execute(
        "DELETE FROM invites WHERE expires_at < ?1 OR (used_at IS NOT NULL AND used_at < ?2)",
        (now, now - 7 * 86_400),
    )
    .await?;
    Ok(())
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
            "SELECT u.id, u.email, u.role, u.password_hash, u.disabled_at, u.created_at
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
