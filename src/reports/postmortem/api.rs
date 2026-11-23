//! Post-mortem routes (v1.2 Q63). Contract: docs/api-specs/05-incidents.md and 03-projects.md.

use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use serde::Deserialize;

use super::{Action, Move, PostmortemError, Text};
use crate::{
    accounts::CurrentUser,
    app::AppState,
    platform::{
        now,
        ui::{Body, UiError, UiResult, ViewerOffset, data},
    },
    projects,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/ui/projects/{slug}/incidents/{id}/postmortem",
            get(read).put(write).delete(discard),
        )
        .route(
            "/api/ui/projects/{slug}/incidents/{id}/postmortem/edit",
            post(edit),
        )
        .route(
            "/api/ui/projects/{slug}/postmortem-recipients",
            get(recipients).put(save_recipients),
        )
}

fn refused(e: PostmortemError) -> UiError {
    match e {
        PostmortemError::Invalid(bad) => {
            UiError::fields(bad.into_iter().map(|(f, m)| (f.to_string(), m)).collect())
        }
        PostmortemError::NotFound => UiError::NotFound,
        PostmortemError::Emails(e) => UiError::field("emails", e.to_string()),
        PostmortemError::Other(e) => UiError::Internal(e),
        e @ (PostmortemError::IncidentOpen | PostmortemError::NotAllowed(_)) => {
            UiError::invalid(e.to_string())
        }
    }
}

async fn read(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, id)): Path<(String, i64)>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    crate::incidents::find(&state.db, p.id, id)
        .await?
        .ok_or(UiError::NotFound)?;
    Ok(data(super::view(&state.db, p.id, id).await?))
}

/// `action` is `save` or `publish`; the sections ride beside it.
#[derive(Deserialize, Default)]
#[serde(default)]
struct Form {
    action: String,
    #[serde(flatten)]
    text: Text,
}

async fn apply(
    state: &AppState,
    user: &crate::accounts::User,
    slug: &str,
    id: i64,
    action: Action,
    text: &Text,
    off: i64,
) -> UiResult {
    let p = projects::find_for(&state.db, user, slug).await?;
    let mv = Move {
        action,
        text,
        by: &user.email,
    };
    let out = super::apply(&state.db, &p, id, mv, now(), off)
        .await
        .map_err(refused)?;
    Ok(data(out))
}

async fn write(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, id)): Path<(String, i64)>,
    ViewerOffset(off): ViewerOffset,
    Body(f): Body<Form>,
) -> UiResult {
    let action = match f.action.as_str() {
        "save" => Action::Save,
        "publish" => Action::Publish,
        _ => return Err(UiError::field("action", "Pick save or publish.")),
    };
    apply(&state, &user, &slug, id, action, &f.text, off).await
}

async fn edit(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, id)): Path<(String, i64)>,
    ViewerOffset(off): ViewerOffset,
) -> UiResult {
    apply(
        &state,
        &user,
        &slug,
        id,
        Action::Edit,
        &Text::default(),
        off,
    )
    .await
}

async fn discard(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, id)): Path<(String, i64)>,
    ViewerOffset(off): ViewerOffset,
) -> UiResult {
    apply(
        &state,
        &user,
        &slug,
        id,
        Action::Discard,
        &Text::default(),
        off,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn recipients(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    Ok(data(super::recipients(&state.db, p.id).await?))
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct ListForm {
    enabled: bool,
    emails: String,
}

async fn save_recipients(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
    Body(f): Body<ListForm>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    super::save_recipients(&state.db, p.id, f.enabled, &f.emails)
        .await
        .map_err(refused)?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
