//! Settings, Secrets as JSON. No response carries a value.

use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, put},
};
use serde::Deserialize;

use super::SecretError;
use crate::{
    accounts::CurrentUser,
    app::AppState,
    platform::ui::{Body, UiError, UiResult, created, data},
    projects,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/ui/projects/{slug}/secrets", get(list).post(create))
        .route(
            "/api/ui/projects/{slug}/secrets/{id}",
            put(replace).delete(delete),
        )
}

impl From<SecretError> for UiError {
    fn from(e: SecretError) -> Self {
        match e {
            SecretError::BadName | SecretError::NameTaken => UiError::field("name", e.to_string()),
            SecretError::BadValue => UiError::field("value", e.to_string()),
            SecretError::Full | SecretError::InUse(_) => UiError::invalid(e.to_string()),
            SecretError::NotFound => UiError::NotFound,
            SecretError::Other(e) => e.into(),
        }
    }
}

async fn list(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    Ok(data(super::list(&state.db, p.id).await?))
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct SecretForm {
    name: String,
    value: String,
}

/// 201 `{id}`; the value is not echoed.
async fn create(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
    Body(f): Body<SecretForm>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    let id = super::create(&state.db, p.id, &f.name, &f.value).await?;
    Ok(created(serde_json::json!({ "id": id })))
}

async fn replace(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, id)): Path<(String, i64)>,
    Body(f): Body<SecretForm>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    super::replace(&state.db, p.id, id, &f.value).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn delete(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, id)): Path<(String, i64)>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    super::delete(&state.db, p.id, id).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
