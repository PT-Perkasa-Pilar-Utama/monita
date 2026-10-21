//! SQL for Monita's Telegram bot: its settings rows and the connect links.

use turso::Connection;

use crate::platform::settings;

const TOKEN: &str = "telegram_bot_token";
const NAME: &str = "telegram_bot_name";
const OFFSET: &str = "telegram_offset";

/// The bot's token and `@name`, when a super admin set one.
pub async fn bot(c: &Connection) -> anyhow::Result<Option<(String, String)>> {
    let token = settings::get_raw(c, TOKEN).await?.filter(|t| !t.is_empty());
    let name = settings::get_raw(c, NAME).await?.unwrap_or_default();
    Ok(token.map(|t| (t, name)))
}

/// Saves the bot, or forgets it for `None`. A new bot starts reading its messages from the start.
pub async fn set_bot(c: &Connection, bot: Option<(&str, &str)>) -> anyhow::Result<()> {
    let (token, name) = bot.unwrap_or(("", ""));
    settings::set_raw(c, TOKEN, token).await?;
    settings::set_raw(c, NAME, name).await?;
    settings::set_raw(c, OFFSET, "0").await
}

/// The first update the bot has not read yet.
pub async fn offset(c: &Connection) -> anyhow::Result<i64> {
    Ok(settings::get_raw(c, OFFSET)
        .await?
        .and_then(|v| v.parse().ok())
        .unwrap_or(0))
}

/// Moves the offset forward only, so two checks at once never read an update twice.
pub async fn advance_offset(c: &Connection, next: i64) -> anyhow::Result<()> {
    if next > offset(c).await? {
        settings::set_raw(c, OFFSET, &next.to_string()).await?;
    }
    Ok(())
}

/// A new link for one severity. Its older links and every expired link go.
pub async fn new_link(
    c: &Connection,
    code_hash: &str,
    project_id: i64,
    severity: &str,
    expires_at: i64,
    now: i64,
) -> anyhow::Result<()> {
    c.execute(
        "DELETE FROM telegram_links WHERE (project_id = ?1 AND severity = ?2) OR expires_at <= ?3",
        (project_id, severity, now),
    )
    .await?;
    c.execute(
        "INSERT INTO telegram_links (code_hash, project_id, severity, expires_at) VALUES (?1, ?2, ?3, ?4)",
        (code_hash, project_id, severity, expires_at),
    )
    .await?;
    Ok(())
}

/// Marks the live link with this code as heard in a chat. False when no live link has it.
pub async fn heard(
    c: &Connection,
    code_hash: &str,
    chat_id: &str,
    title: &str,
    now: i64,
) -> anyhow::Result<bool> {
    let n = c
        .execute(
            "UPDATE telegram_links SET chat_id = ?2, chat_title = ?3 WHERE code_hash = ?1 AND expires_at > ?4",
            (code_hash, chat_id, title, now),
        )
        .await?;
    Ok(n > 0)
}

/// The chat a severity's live link was heard in, which uses the link up.
pub async fn take_heard(
    c: &Connection,
    project_id: i64,
    severity: &str,
    now: i64,
) -> anyhow::Result<Option<(String, String)>> {
    let found = crate::platform::db::query_all(
        c,
        "SELECT chat_id, chat_title FROM telegram_links
         WHERE project_id = ?1 AND severity = ?2 AND expires_at > ?3 AND chat_id IS NOT NULL LIMIT 1",
        (project_id, severity, now),
        |r| Ok((r.get::<String>(0)?, r.get::<String>(1)?)),
    )
    .await?
    .pop();
    if found.is_some() {
        c.execute(
            "DELETE FROM telegram_links WHERE project_id = ?1 AND severity = ?2",
            (project_id, severity),
        )
        .await?;
    }
    Ok(found)
}
