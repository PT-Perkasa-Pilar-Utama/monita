//! Monita's Telegram bot as JSON: the admin setting and a severity's connect link.
//! Contract: docs/api-specs/06-notify.md section 6.4.

use axum::{
    Router,
    extract::{Path, State},
    routing::{get, post, put},
};
use serde::Deserialize;
use serde_json::json;

use super::TelegramError;
use crate::{
    accounts::{AdminUser, CurrentUser},
    app::AppState,
    platform::{
        now,
        ui::{Body, UiError, UiResult, data},
    },
    projects,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/ui/telegram", get(bot))
        .route("/api/ui/admin/telegram", put(save))
        .route("/api/ui/projects/{slug}/telegram-link", post(link))
        .route("/api/ui/projects/{slug}/telegram-link/check", post(check))
}

impl From<TelegramError> for UiError {
    fn from(e: TelegramError) -> Self {
        match e {
            TelegramError::BadToken | TelegramError::WebhookSet => {
                UiError::field("token", e.to_string())
            }
            TelegramError::NoBot | TelegramError::Unreachable => UiError::invalid(e.to_string()),
            TelegramError::Other(e) => UiError::Internal(e),
        }
    }
}

/// `{bot}`: the bot's name without `@`, or null. Never the token.
async fn bot(State(state): State<AppState>, CurrentUser(_): CurrentUser) -> UiResult {
    Ok(data(json!({ "bot": super::bot_name(&state.db).await? })))
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct TokenForm {
    token: String,
}

/// A super admin sets or clears the bot. `AdminUser` lets a supervisor read only, so the PUT
/// refuses them.
async fn save(
    State(state): State<AppState>,
    AdminUser(user): AdminUser,
    Body(f): Body<TokenForm>,
) -> UiResult {
    if !user.is_admin() {
        return Err(UiError::Forbidden);
    }
    let name = super::set_bot(
        &state.db,
        state.http.webhook(),
        &state.config.telegram_api,
        &f.token,
    )
    .await?;
    Ok(data(json!({ "bot": name })))
}

async fn link(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    Ok(data(super::new_link(&state.db, p.id, now()).await?))
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct CodeForm {
    code: String,
}

/// `{chat}`: the chat the link with `code` was opened in, or null while nobody has opened it.
async fn check(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
    Body(f): Body<CodeForm>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    let chat = super::check_link(
        &state.db,
        state.http.webhook(),
        &state.config.telegram_api,
        p.id,
        &f.code,
        now(),
    )
    .await?;
    Ok(data(json!({ "chat": chat })))
}
