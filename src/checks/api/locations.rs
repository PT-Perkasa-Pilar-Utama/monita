//! The Locations admin page's JSON (technical-specs 14.8, api-specs 04 section 4.5). A super
//! admin manages; a supervisor reads, and `CurrentUser` refuses its writes.

use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
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
        .route("/api/ui/admin/locations/{id}/token", post(rotate))
        .route("/api/ui/admin/locations/{id}/monitors", get(monitors))
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

/// A new token for the location, shown once; the old one stops working at once.
async fn rotate(State(state): State<AppState>, _: AdminUser, Path(id): Path<i64>) -> UiResult {
    match locations::rotate(&state.db, id).await? {
        Some(token) => Ok(data(serde_json::json!({ "token": token }))),
        None => Err(UiError::NotFound),
    }
}

/// The monitors that check from the location, each with its project for a link.
async fn monitors(State(state): State<AppState>, _: AdminUser, Path(id): Path<i64>) -> UiResult {
    let rows = locations::monitors_of(&state.db, id).await?;
    let ids: Vec<i64> = rows.iter().map(|r| r.1).collect();
    let projects = crate::projects::by_ids(&state.db, &ids).await?;
    let list: Vec<serde_json::Value> = rows
        .into_iter()
        .filter_map(|(mid, pid, name)| {
            let p = projects.get(&pid)?;
            Some(serde_json::json!({ "id": mid, "name": name, "project": p.name, "slug": p.slug }))
        })
        .collect();
    Ok(data(
        serde_json::json!({ "monitors": list, "shown_max": caps::LOCATION_MONITORS_SHOWN }),
    ))
}

async fn remove(State(state): State<AppState>, _: AdminUser, Path(id): Path<i64>) -> UiResult {
    if locations::delete(&state.db, id).await? {
        Ok(StatusCode::NO_CONTENT.into_response())
    } else {
        Err(UiError::NotFound)
    }
}
