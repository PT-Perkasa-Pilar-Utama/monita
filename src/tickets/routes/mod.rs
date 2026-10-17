//! Ticket API and pages' JSON. Contract: docs/api-specs/08-tickets.md.
//! `api` serves project backends, `public` the reporter and visitors, `board` the tech lead.

mod api;
mod board;
mod public;

use axum::{
    Json, Router,
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use serde::Deserialize;

use crate::tickets::{NewTicket, TicketError};
use crate::{
    accounts::CurrentUser,
    app::AppState,
    platform::{
        api_error,
        ui::{Body, UiError, UiResult, created, data},
    },
    projects::{self, ApiProject},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/ref-ids", post(api::issue_ref_id))
        .route(
            "/api/v1/tickets",
            post(api::api_submit).options(api::preflight),
        )
        .route(
            "/api/v1/tickets/mine",
            get(api::api_mine).options(api::preflight),
        )
        .route("/api/ui/public/{slug}", get(public::project))
        .route("/api/ui/public/{slug}/tickets", get(public::public_board))
        .route("/api/ui/public/{slug}/report", post(public::report))
        .route("/api/ui/public/{slug}/mine", post(public::mine))
        .route("/api/ui/projects/{slug}/tickets", get(board::board))
        .route(
            "/api/ui/projects/{slug}/tickets/{number}",
            get(board::detail),
        )
        .route(
            "/api/ui/projects/{slug}/tickets/{number}/stage",
            put(board::move_stage),
        )
        .route(
            "/api/ui/projects/{slug}/tickets/{number}/public-title",
            put(board::public_title),
        )
        .route(
            "/api/ui/projects/{slug}/tickets/{number}/revoke-ref",
            post(board::revoke_ref),
        )
}

impl From<TicketError> for UiError {
    fn from(e: TicketError) -> Self {
        match e {
            TicketError::Invalid { field, message } => UiError::field(field, message),
            TicketError::RefIdInvalid => UiError::field("ref_id", e.to_string()),
            TicketError::NeedReason => UiError::field("close_reason", e.to_string()),
            TicketError::DailyCap => UiError::DailyCap(e.to_string()),
            TicketError::Other(e) => UiError::Internal(e),
        }
    }
}

/// Stage name to its ticket count; a stage with none is 0.
fn counts(rows: Vec<(String, i64)>) -> serde_json::Map<String, serde_json::Value> {
    crate::tickets::STAGES
        .iter()
        .map(|(k, _)| {
            let n = rows.iter().find(|(s, _)| s == k).map_or(0, |(_, n)| *n);
            (k.to_string(), n.into())
        })
        .collect()
}
