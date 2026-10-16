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

/// Returns `None` when the email is taken.
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

pub async fn email_of(c: &Connection, user_id: i64) -> anyhow::Result<Option<String>> {
    let mut rows = c
        .query("SELECT email FROM users WHERE id = ?1", [user_id])
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

pub async fn disable_user(c: &Connection, user_id: i64, at: i64) -> anyhow::Result<()> {
    c.execute(
        "UPDATE users SET disabled_at = ?1, updated_at = ?1 WHERE id = ?2",
        (at, user_id),
    )
    .await?;
    c.execute("DELETE FROM sessions WHERE user_id = ?1", [user_id])
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
            "SELECT id, user_id, expires_at, used_at FROM invites WHERE token_hash = ?1",
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
        "UPDATE users SET password_hash = ?1, disabled_at = NULL, updated_at = ?2 WHERE id = ?3",
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
