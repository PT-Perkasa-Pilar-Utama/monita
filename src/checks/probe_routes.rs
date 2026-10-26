//! The API a location's `monita probe` calls: claim due checks, report their results
//! (technical-specs 14, api-specs 04 section 4.5).

use axum::{
    Router,
    extract::{FromRequestParts, Query, State},
    http::{StatusCode, header, request::Parts},
    response::Response,
    routing::{get, post},
};
use serde::Deserialize;

use super::locations::{self, Reported};
use crate::{
    app::AppState,
    platform::{
        api_error, caps, now,
        rate_limit::{Verdict, too_many},
        ui::{Body, UiError, UiResult, data},
    },
};

/// Calls a minute one location may make (technical-specs 14.4). A probe polling each second uses 60.
const PER_MIN: u32 = 300;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/probe/work", get(work))
        .route("/api/v1/probe/results", post(results))
}

/// The location whose token signed the request, and when it last called.
pub struct ProbeLocation {
    id: i64,
    last_seen_at: Option<i64>,
}

impl FromRequestParts<AppState> for ProbeLocation {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let unauthorized = || {
            api_error(
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "Missing or invalid location token.",
            )
        };
        let internal = |e: anyhow::Error| {
            tracing::error!(error = %e, "location token lookup failed");
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal",
                "Unexpected failure.",
            )
        };
        let raw = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split_once(' '))
            .filter(|(scheme, _)| scheme.eq_ignore_ascii_case("bearer"))
            .map(|(_, token)| token)
            .ok_or_else(unauthorized)?;
        let conn = state.db.read().await.map_err(internal)?;
        // An unknown token is not limited: tokens are random, so guessing one is hopeless (Q45).
        let Some((id, last_seen_at)) = locations::authenticate(&conn, raw)
            .await
            .map_err(internal)?
        else {
            return Err(unauthorized());
        };
        drop(conn);
        if let Verdict::Deny(retry) = state
            .limiter
            .hit(&format!("probe:{id}"), PER_MIN, 60, now())
        {
            return Err(too_many(parts.uri.path(), retry));
        }
        Ok(ProbeLocation { id, last_seen_at })
    }
}

#[derive(Deserialize)]
struct WorkQuery {
    free: Option<String>,
}

async fn work(
    State(state): State<AppState>,
    at: ProbeLocation,
    Query(q): Query<WorkQuery>,
) -> UiResult {
    let free = match q.free.as_deref().map(str::trim) {
        None | Some("") => caps::PROBE_BATCH,
        Some(n) => n
            .parse::<i64>()
            .ok()
            .filter(|n| (1..=caps::PROBE_BATCH).contains(n))
            .ok_or_else(|| UiError::field("free", "Ask for 1 to 50 jobs."))?,
    };
    let jobs = locations::claim_work(&state.db, at.id, at.last_seen_at, free).await?;
    Ok(data(serde_json::json!({ "jobs": jobs })))
}

#[derive(Deserialize)]
struct ResultsBody {
    results: Vec<Reported>,
}

async fn results(
    State(state): State<AppState>,
    at: ProbeLocation,
    Body(body): Body<ResultsBody>,
) -> UiResult {
    let n = body.results.len() as i64;
    if !(1..=caps::PROBE_BATCH).contains(&n) {
        return Err(UiError::field("results", "Send 1 to 50 results."));
    }
    let (stored, skipped) = locations::post_results(&state.db, at.id, body.results).await?;
    Ok(data(
        serde_json::json!({ "stored": stored, "skipped": skipped }),
    ))
}
