//! Ticket API. Contract: docs/api-specs/08-tickets.md.

use axum::{
    Json, Router,
    body::Bytes,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
};
use serde::Deserialize;

use crate::{
    app::AppState,
    projects::{ApiProject, api_error},
};

pub fn router() -> Router<AppState> {
    Router::new().route("/api/v1/ref-ids", post(issue_ref_id))
}

#[derive(Deserialize, Default)]
struct RefIdRequest {
    #[serde(default)]
    single_use: bool,
}

async fn issue_ref_id(
    State(state): State<AppState>,
    ApiProject(project): ApiProject,
    body: Bytes,
) -> Response {
    // The body is optional: no body, `null` and `{}` all mean the defaults.
    let req = if body.iter().all(u8::is_ascii_whitespace) {
        RefIdRequest::default()
    } else {
        match serde_json::from_slice::<Option<RefIdRequest>>(&body) {
            Ok(r) => r.unwrap_or_default(),
            Err(_) => {
                return api_error(
                    StatusCode::BAD_REQUEST,
                    "invalid_request",
                    "Body must be JSON like {\"single_use\": true}.",
                );
            }
        }
    };
    let single_use = req.single_use;
    match super::issue_ref_id(&state.db, project.id, single_use).await {
        Ok((ref_id, created_at)) => (
            StatusCode::CREATED,
            Json(serde_json::json!({ "data": { "ref_id": ref_id, "single_use": single_use, "created_at": created_at } })),
        )
            .into_response(),
        Err(e) => {
            tracing::error!(error = %e, "issue ref id failed");
            api_error(StatusCode::INTERNAL_SERVER_ERROR, "internal", "Unexpected failure.")
        }
    }
}
