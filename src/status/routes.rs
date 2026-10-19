//! Public status page and JSON. Contract: docs/api-specs/07-status.md.

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Deserialize;

use crate::{
    app::AppState,
    platform::{
        api_error, caps,
        ui::{UiError, UiResult, data},
    },
};

const DAYS: usize = 90;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/ui/public/{slug}/status", get(page))
        .route("/api/ui/public/{slug}/history", get(history))
        .route("/api/ui/public/{slug}/incidents", get(incidents_by_month))
        .route("/api/ui/public/{slug}/uptime", get(uptime_calendar))
        .route("/api/v1/public/status/{slug}", get(json))
}

#[derive(Deserialize)]
struct DaysQuery {
    days: Option<String>,
}

async fn json(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(q): Query<DaysQuery>,
) -> Response {
    let days = match q.days.as_deref().map(str::parse::<usize>) {
        None => DAYS,
        Some(Ok(d)) if (1..=DAYS).contains(&d) => d,
        Some(_) => {
            return api_error(
                StatusCode::BAD_REQUEST,
                "invalid_request",
                "days must be a whole number from 1 to 90.",
            );
        }
    };
    let mut res = match super::load(&state.db, &slug, days).await {
        Ok(Some(status)) => crate::platform::ui::public_cache(
            Json(serde_json::json!({ "data": status })).into_response(),
        ),
        Ok(None) => api_error(
            StatusCode::NOT_FOUND,
            "not_found",
            "No status page with this slug.",
        ),
        Err(e) => {
            tracing::error!(error = %e, "status json failed");
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal",
                "Unexpected failure.",
            )
        }
    };
    res.headers_mut().insert(
        header::ACCESS_CONTROL_ALLOW_ORIGIN,
        HeaderValue::from_static("*"),
    );
    res
}

/// The status page: 90 days of public items, every open incident, and the resolved ones of the
/// last 7 days. Wire types are the status module's own (SEC-21).
async fn page(State(state): State<AppState>, Path(slug): Path<String>) -> UiResult {
    let d = super::page(&state.db, &slug)
        .await?
        .ok_or(UiError::NotFound)?;
    let body = serde_json::json!({ "status": d.status, "recent": d.recent });
    Ok(crate::platform::ui::public_cache(data(body)))
}

#[derive(Deserialize)]
struct Before {
    /// `<started_at>.<id>`, a cursor from `older`; a bad value reads the first page.
    before: Option<String>,
}

async fn history(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(q): Query<Before>,
) -> UiResult {
    let cursor = q.before.as_deref().and_then(|b| {
        let (t, id) = b.split_once('.')?;
        Some((t.parse::<i64>().ok()?, id.parse::<i64>().ok()?))
    });
    let (project, incidents) = super::history_page(&state.db, &slug, cursor)
        .await?
        .ok_or(UiError::NotFound)?;
    let older = (incidents.len() as i64 == caps::PUBLIC_HISTORY_PAGE)
        .then(|| {
            incidents
                .last()
                .map(|i| format!("{}.{}", i.started_at, i.id))
        })
        .flatten();
    let body = serde_json::json!({ "project": project, "incidents": incidents, "older": older });
    Ok(crate::platform::ui::public_cache(data(body)))
}

/// A window query: `to` is any moment in the window's last month; a bad value reads as now.
#[derive(Deserialize)]
struct WindowQuery {
    to: Option<String>,
    kind: Option<String>,
    id: Option<String>,
}

impl WindowQuery {
    fn to(&self) -> Option<i64> {
        self.to.as_deref().and_then(|t| t.parse().ok())
    }
}

/// Three months of public incidents, newest month first. 404 with the Incidents tab off.
async fn incidents_by_month(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(q): Query<WindowQuery>,
) -> UiResult {
    let page = super::months::by_month(&state.db, &slug, q.to())
        .await?
        .ok_or(UiError::NotFound)?;
    Ok(crate::platform::ui::public_cache(data(page)))
}

/// Three months of one public system's days. A system that is not public is not found.
async fn uptime_calendar(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(q): Query<WindowQuery>,
) -> UiResult {
    let pick = match (q.kind.as_deref(), q.id.as_deref()) {
        (None, None) => None,
        (Some(kind), Some(id)) => Some((
            crate::incidents::SourceKind::parse(kind).ok_or(UiError::NotFound)?,
            id.parse::<i64>().map_err(|_| UiError::NotFound)?,
        )),
        _ => return Err(UiError::NotFound),
    };
    let page = super::uptime_calendar::calendar(&state.db, &slug, pick, q.to())
        .await?
        .ok_or(UiError::NotFound)?;
    Ok(crate::platform::ui::public_cache(data(page)))
}
