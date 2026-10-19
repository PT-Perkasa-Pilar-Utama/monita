//! Ticket API and pages' JSON. Contract: docs/api-specs/08-tickets.md.
//! `api` serves project backends, `public` the reporter and visitors, `board` the tech lead.

mod api;
mod board;
mod public;

use axum::{
    Json, Router,
    body::Bytes,
    extract::{Path, Query, State},
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
        .route(
            "/api/ui/projects/{slug}/tickets",
            get(board::board).post(board::file),
        )
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
        .route("/api/ui/projects/{slug}/ref-ids", post(board::issue_ref_id))
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

/// `?stage=&page=` asks for one more page of one column instead of the whole board.
#[derive(Deserialize, Default)]
#[serde(default)]
pub(super) struct ColumnQuery {
    stage: Option<String>,
    page: Option<i64>,
}

impl ColumnQuery {
    /// The column asked for; an unknown stage is a 404, like an unknown page.
    fn stage(&self) -> Result<Option<&'static str>, UiError> {
        match &self.stage {
            None => Ok(None),
            Some(s) => crate::tickets::STAGES
                .iter()
                .find(|k| *k == s)
                .map(|k| Some(*k))
                .ok_or(UiError::NotFound),
        }
    }
}

/// Every ticket in one column, for a page's "of N".
async fn stage_total(state: &AppState, project_id: i64, stage: &str) -> Result<i64, UiError> {
    let rows = crate::tickets::stage_counts(&state.db, project_id).await?;
    Ok(rows.iter().find(|(s, _)| s == stage).map_or(0, |(_, n)| *n))
}

/// Stage name to its ticket count; a stage with none is 0.
fn counts(rows: Vec<(String, i64)>) -> serde_json::Map<String, serde_json::Value> {
    crate::tickets::STAGES
        .iter()
        .map(|k| {
            let n = rows.iter().find(|(s, _)| s == k).map_or(0, |(_, n)| *n);
            (k.to_string(), n.into())
        })
        .collect()
}
