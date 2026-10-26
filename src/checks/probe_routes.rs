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
    /// Seconds to hold the request while nothing is due, 0 to `caps::PROBE_WAIT_S`. Missing is 0,
    /// as probes before long polling ask.
    wait: Option<String>,
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
    let wait = match q.wait.as_deref().map(str::trim) {
        None | Some("") => 0,
        Some(n) => n
            .parse::<u64>()
            .ok()
            .filter(|n| *n <= caps::PROBE_WAIT_S)
            .ok_or_else(|| {
                UiError::field("wait", format!("Wait 0 to {} seconds.", caps::PROBE_WAIT_S))
            })?,
    };
    // Long poll: hold the request until a check is due, the wait runs out or shutdown starts, so
    // an idle probe calls about twice a minute, not every second.
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(wait);
    let mut stopping = state.stopping.subscribe();
    let mut seen = at.last_seen_at;
    loop {
        let jobs = locations::claim_work(&state.db, at.id, seen, free).await?;
        let t = now();
        if seen.is_none_or(|s| t - s >= 30) {
            seen = Some(t);
        }
        let left = deadline.saturating_duration_since(tokio::time::Instant::now());
        if !jobs.is_empty() || left.is_zero() || *stopping.borrow() {
            return Ok(data(serde_json::json!({ "jobs": jobs })));
        }
        let nap = locations::nap(&state.db, at.id).await?.min(left);
        tokio::select! {
            _ = tokio::time::sleep(nap) => {}
            _ = stopping.wait_for(|s| *s) => {}
        }
    }
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
