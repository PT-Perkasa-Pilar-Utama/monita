//! The Locations admin page's JSON (technical-specs 14.8, api-specs 04 section 4.5). A super
//! admin manages; a supervisor reads, and `CurrentUser` refuses its writes.

use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get},
};
use serde::Deserialize;

use crate::{
    accounts::AdminUser,
    app::AppState,
    checks::locations::{self, LocationError},
    platform::{
        caps,
        ui::{Body, UiError, UiResult, created, data},
    },
};

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/api/ui/admin/locations", get(list).post(create))
        .route("/api/ui/admin/locations/{id}", delete(remove))
}

async fn list(State(state): State<AppState>, _: AdminUser) -> UiResult {
    let rows = locations::list(&state.db).await?;
    Ok(data(
        serde_json::json!({ "locations": rows, "max": caps::LOCATIONS }),
    ))
}

#[derive(Deserialize)]
struct NewLocation {
    #[serde(default)]
    name: String,
}

async fn create(
    State(state): State<AppState>,
    _: AdminUser,
    Body(form): Body<NewLocation>,
) -> UiResult {
    match locations::create(&state.db, &form.name).await {
        Ok((id, token)) => Ok(created(
            serde_json::json!({ "id": id, "name": form.name.trim(), "token": token }),
        )),
        Err(LocationError::Other(e)) => Err(UiError::Internal(e)),
        Err(e) => Err(UiError::field("name", e.to_string())),
    }
}

async fn remove(State(state): State<AppState>, _: AdminUser, Path(id): Path<i64>) -> UiResult {
    if locations::delete(&state.db, id).await? {
        Ok(StatusCode::NO_CONTENT.into_response())
    } else {
        Err(UiError::NotFound)
    }
}
