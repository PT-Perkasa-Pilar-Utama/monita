//! Monita's own Telegram bot (v1.1 Q33). A super admin sets its token once. A project's severity
//! then uses the `telegram_bot` format with a chat that a connect link found: the owner opens
//! `t.me/<bot>?start=<code>`, or adds the bot to a group with `?startgroup=<code>`; the bot hears
//! `/start <code>`; the page's check reads the bot's messages and fills in the chat ID.
//! Contract: docs/api-specs/06-notify.md section 6.4.

pub mod api;
mod store;

use std::time::Duration;

use serde::Serialize;
use serde_json::Value;

use crate::platform::{db::Db, token};

/// How long a connect link waits for its `/start` (v1.1 Q33, as Q32's two-factor key).
pub const LINK_TTL_S: i64 = 600;
const CALL_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, thiserror::Error)]
pub enum TelegramError {
    #[error("Telegram refused this token. Copy it again from @BotFather.")]
    BadToken,
    #[error(
        "This bot sends its messages to a webhook, so Monita cannot read them. Use a bot for Monita alone."
    )]
    WebhookSet,
    #[error("Ask a super admin to set Monita's Telegram bot first.")]
    NoBot,
    #[error("Monita could not reach Telegram. Try again.")]
    Unreachable,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// A connect link, with the private chat link as a QR code for the phone.
#[derive(Serialize)]
pub struct Link {
    pub private: String,
    pub group: String,
    pub qr: String,
    pub expires_at: i64,
}

/// The chat a link was opened in.
#[derive(Serialize, Debug, PartialEq)]
pub struct Chat {
    pub chat_id: String,
    pub title: String,
}

/// The bot's `@name`, for pages; never the token.
pub async fn bot_name(db: &Db) -> anyhow::Result<Option<String>> {
    Ok(store::bot(&db.read().await?).await?.map(|(_, n)| n))
}

/// Where `telegram_bot` alerts post, built at delivery so no route or outbox row holds the token.
pub async fn send_url(db: &Db, api: &str) -> anyhow::Result<Option<String>> {
    Ok(store::bot(&db.read().await?)
        .await?
        .map(|(t, _)| format!("{api}/bot{t}/sendMessage")))
}

/// Posts one alert. A refusal carries Telegram's reason, such as "chat not found", never the URL.
pub async fn post(client: &reqwest::Client, url: &str, body: &str) -> anyhow::Result<()> {
    let res = client
        .post(url)
        .header("content-type", "application/json")
        .body(body.to_string())
        .timeout(CALL_TIMEOUT)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("{}", e.without_url()))?;
    let status = res.status();
    if status.is_success() {
        return Ok(());
    }
    let why = res
        .bytes()
        .await
        .ok()
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        .and_then(|v| v["description"].as_str().map(str::to_string))
        .unwrap_or_default();
    anyhow::bail!("Telegram answered HTTP {}: {why}", status.as_u16())
}

/// A token as @BotFather gives it: `123456:ABC-def_ghi`. It goes into a URL path, so nothing
/// else passes.
fn token_shape(t: &str) -> bool {
    let (id, secret) = t.split_once(':').unwrap_or_default();
    !id.is_empty()
        && id.len() <= 20
        && id.chars().all(|c| c.is_ascii_digit())
        && (20..=80).contains(&secret.len())
        && secret
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// One Bot API call. `query` is already URL-safe. Errors never quote the URL: it holds the token.
async fn call(
    client: &reqwest::Client,
    api: &str,
    token: &str,
    method: &str,
    query: &str,
) -> Result<(u16, Value), TelegramError> {
    let res = client
        .get(format!("{api}/bot{token}/{method}?{query}"))
        .timeout(CALL_TIMEOUT)
        .send()
        .await
        .map_err(|e| {
            tracing::warn!(error = %e.without_url(), method, "Telegram call failed");
            TelegramError::Unreachable
        })?;
    let status = res.status().as_u16();
    let bytes = res.bytes().await.map_err(|e| {
        tracing::warn!(error = %e.without_url(), method, "Telegram answer cut short");
        TelegramError::Unreachable
    })?;
    let body = serde_json::from_slice::<Value>(&bytes).map_err(|_| {
        tracing::warn!(method, status, "Telegram answered without JSON");
        TelegramError::Unreachable
    })?;
    Ok((status, body))
}

/// Saves Monita's bot after Telegram confirms the token; an empty token forgets the bot.
/// Returns the bot's name.
pub async fn set_bot(
    db: &Db,
    client: &reqwest::Client,
    api: &str,
    token: &str,
) -> Result<Option<String>, TelegramError> {
    let token = token.trim();
    if token.is_empty() {
        db.write(async |c| store::set_bot(c, None).await).await?;
        return Ok(None);
    }
    if !token_shape(token) {
        return Err(TelegramError::BadToken);
    }
    let (status, me) = call(client, api, token, "getMe", "").await?;
    if matches!(status, 401 | 404) {
        return Err(TelegramError::BadToken);
    }
    let Some(name) = me["result"]["username"].as_str().map(str::to_string) else {
        return Err(anyhow::anyhow!("getMe answered HTTP {status} without a username").into());
    };
    let (_, hook) = call(client, api, token, "getWebhookInfo", "").await?;
    if hook["result"]["url"]
        .as_str()
        .is_some_and(|u| !u.is_empty())
    {
        return Err(TelegramError::WebhookSet);
    }
    db.write(async |c| store::set_bot(c, Some((token, &name))).await)
        .await?;
    Ok(Some(name))
}

/// A new connect link for one severity of a project.
pub async fn new_link(
    db: &Db,
    project_id: i64,
    severity: &str,
    now: i64,
) -> Result<Link, TelegramError> {
    let name = bot_name(db).await?.ok_or(TelegramError::NoBot)?;
    let code = token::random_token();
    let hash = token::hash(&code);
    let expires_at = now + LINK_TTL_S;
    db.write(async |c| store::new_link(c, &hash, project_id, severity, expires_at, now).await)
        .await?;
    let private = format!("https://t.me/{name}?start={code}");
    Ok(Link {
        qr: crate::platform::qr::svg(&private)?,
        group: format!("https://t.me/{name}?startgroup={code}"),
        private,
        expires_at,
    })
}

/// The `<code>` of `/start <code>`, or of `/start@Bot <code>` as a group sends it to `bot`.
fn parse_start<'a>(text: &'a str, bot: &str) -> Option<&'a str> {
    let rest = text.trim().strip_prefix("/start")?;
    let rest = if let Some(r) = rest.strip_prefix('@') {
        let (name, after) = r.split_once(char::is_whitespace)?;
        name.eq_ignore_ascii_case(bot).then_some(after)?
    } else if rest.starts_with(char::is_whitespace) {
        rest
    } else {
        return None;
    };
    let code = rest.trim();
    let ok = !code.is_empty()
        && code.len() <= 64
        && code
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    ok.then_some(code)
}

/// What a chat is called: a group's title, a person's `@username`, or their first name.
fn chat_title(chat: &Value) -> String {
    let id = chat["id"].as_i64().unwrap_or_default();
    chat["title"]
        .as_str()
        .map(str::to_string)
        .or_else(|| chat["username"].as_str().map(|u| format!("@{u}")))
        .or_else(|| chat["first_name"].as_str().map(str::to_string))
        .unwrap_or_else(|| id.to_string())
}

/// The codes the bot heard, with their chats, and the next offset. Every link's code counts, not
/// only this project's: a read moves the offset past them for good.
fn heard_codes(updates: &Value, bot: &str) -> (Vec<(String, Chat)>, Option<i64>) {
    let mut codes = Vec::new();
    let mut next = None;
    for u in updates["result"].as_array().into_iter().flatten() {
        if let Some(id) = u["update_id"].as_i64() {
            next = Some(next.map_or(id + 1, |n: i64| n.max(id + 1)));
        }
        let msg = &u["message"];
        let (Some(text), Some(chat_id)) = (msg["text"].as_str(), msg["chat"]["id"].as_i64()) else {
            continue;
        };
        if let Some(code) = parse_start(text, bot) {
            let chat = Chat {
                chat_id: chat_id.to_string(),
                title: chat_title(&msg["chat"]),
            };
            codes.push((code.to_string(), chat));
        }
    }
    (codes, next)
}

/// Reads the bot's new messages, records every link they open, and returns the chat this
/// severity's link was opened in, if it was.
pub async fn check_link(
    db: &Db,
    client: &reqwest::Client,
    api: &str,
    project_id: i64,
    severity: &str,
    now: i64,
) -> Result<Option<Chat>, TelegramError> {
    let (bot, offset) = {
        let c = db.read().await?;
        (store::bot(&c).await?, store::offset(&c).await?)
    };
    let (token, name) = bot.ok_or(TelegramError::NoBot)?;
    // allowed_updates=["message"], URL-encoded.
    let query = format!("offset={offset}&timeout=0&limit=100&allowed_updates=%5B%22message%22%5D");
    let (status, updates) = call(client, api, &token, "getUpdates", &query).await?;
    match status {
        409 => return Err(TelegramError::WebhookSet),
        401 | 404 => return Err(TelegramError::BadToken),
        s if !(200..300).contains(&s) => {
            return Err(anyhow::anyhow!("getUpdates answered HTTP {s}").into());
        }
        _ => {}
    }
    let (codes, next) = heard_codes(&updates, &name);
    let found = db
        .write(async |c| {
            for (code, chat) in &codes {
                store::heard(c, &token::hash(code), &chat.chat_id, &chat.title, now).await?;
            }
            if let Some(next) = next {
                store::advance_offset(c, next).await?;
            }
            store::take_heard(c, project_id, severity, now).await
        })
        .await?;
    Ok(found.map(|(chat_id, title)| Chat { chat_id, title }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn start_codes_are_read_from_a_private_chat_and_a_group() {
        assert_eq!(
            parse_start("/start abc_12-3", "MonitaBot"),
            Some("abc_12-3")
        );
        assert_eq!(parse_start("  /start   abc  ", "MonitaBot"), Some("abc"));
        assert_eq!(
            parse_start("/start@monitabot abc", "MonitaBot"),
            Some("abc")
        );
        assert_eq!(parse_start("/start@OtherBot abc", "MonitaBot"), None);
        assert_eq!(parse_start("/start", "MonitaBot"), None);
        assert_eq!(parse_start("/startabc", "MonitaBot"), None);
        assert_eq!(parse_start("/start a/b", "MonitaBot"), None);
        assert_eq!(parse_start("hello", "MonitaBot"), None);
    }

    #[test]
    fn every_heard_code_counts_and_the_offset_passes_the_last_update() {
        let updates = json!({ "ok": true, "result": [
            { "update_id": 7, "message": { "text": "/start one", "chat": { "id": 42, "first_name": "Rina", "username": "rina" } } },
            { "update_id": 9, "message": { "text": "/start@MonitaBot two", "chat": { "id": -100, "title": "On call" } } },
            { "update_id": 8, "message": { "text": "hi", "chat": { "id": 42 } } },
            { "update_id": 10, "edited_message": {} },
        ]});
        let (codes, next) = heard_codes(&updates, "MonitaBot");
        assert_eq!(
            codes,
            [
                (
                    "one".to_string(),
                    Chat {
                        chat_id: "42".into(),
                        title: "@rina".into()
                    }
                ),
                (
                    "two".to_string(),
                    Chat {
                        chat_id: "-100".into(),
                        title: "On call".into()
                    }
                ),
            ]
        );
        assert_eq!(next, Some(11));
        assert_eq!(
            heard_codes(&json!({ "ok": true, "result": [] }), "MonitaBot").1,
            None
        );
    }

    #[test]
    fn only_a_botfather_shaped_token_reaches_a_url() {
        assert!(token_shape("123456:ABCdefGHIjklMNOpqrSTUvwxYZ_0123456"));
        assert!(!token_shape("123456:short"));
        assert!(!token_shape("abc:ABCdefGHIjklMNOpqrSTUvwxYZ_0123456"));
        assert!(!token_shape("123456:ABCdefGHIjklMNOpqrSTU/../x0123456"));
        assert!(!token_shape("no colon at all"));
    }
}
