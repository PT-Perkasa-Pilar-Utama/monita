//! Ticket API and pages. Contract: docs/api-specs/08-tickets.md.
//! `api` serves project backends, `public` the reporter and visitors, `admin` the TL.

mod admin;
mod api;
mod public;

use askama::Template;
use axum::{
    Form, Json, Router,
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use serde::Deserialize;

use crate::tickets::{
    CLOSE_REASONS, MyTicket, NewTicket, PublicTicket, STAGES, TicketError, TicketRow,
};
use crate::{
    accounts::CurrentUser,
    app::AppState,
    platform::{
        api_error,
        page::{PageError, Shell},
        render::render,
        time,
    },
    projects::{self, ApiProject, Project},
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
        .route(
            "/status/{slug}/report",
            get(public::report_form).post(public::report_submit),
        )
        .route(
            "/status/{slug}/mine",
            get(public::mine_form).post(public::mine_lookup),
        )
        .route("/status/{slug}/tickets", get(public::public_board))
        .route("/projects/{slug}/tickets", get(admin::kanban))
        .route("/projects/{slug}/tickets/{number}", get(admin::detail))
        .route(
            "/projects/{slug}/tickets/{number}/stage",
            post(admin::move_stage),
        )
        .route(
            "/projects/{slug}/tickets/{number}/public-title",
            post(admin::public_title),
        )
        .route(
            "/projects/{slug}/tickets/{number}/revoke-ref",
            post(admin::revoke_ref),
        )
}
