//! Public status page and JSON. Contract: docs/api-specs/07-status.md.

use askama::Template;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Deserialize;

use super::{Day, IncidentStatus, ItemStatus, Status};
use crate::{
    app::AppState,
    platform::{api_error, page::PageError, render::render, time},
};

const DAYS: usize = 90;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/status/{slug}", get(page))
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
        Ok(Some(status)) => crate::platform::render::public_cache(
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

#[derive(Template)]
#[template(path = "status/page.html")]
struct Page {
    status: Status,
}

impl Page {
    fn headline(&self) -> String {
        let down: Vec<&str> = self
            .status
            .items
            .iter()
            .filter(|i| i.state == "down")
            .map(|i| i.name.as_str())
            .collect();
        match self.status.overall {
            "down" if down.len() == 1 => format!("{} is down", down[0]),
            "down" => format!("{} systems are down", down.len()),
            "issue" => "Some systems have issues".into(),
            _ => "All systems operational".into(),
        }
    }
    fn bar_class(&self, d: &Day) -> &'static str {
        match d.uptime {
            None => "is-none",
            Some(_) if d.downtime_s == 0 => "",
            Some(u) if u >= 99.0 => "is-partial",
            Some(_) => "is-down",
        }
    }
    fn bar_title(&self, d: &Day) -> String {
        match d.uptime {
            None => format!("{}: no data", d.day),
            Some(u) if d.downtime_s == 0 => format!("{}: {u:.2}%", d.day),
            Some(u) => format!("{}: {u:.2}%, down {}", d.day, time::duration(d.downtime_s)),
        }
    }
    fn uptime(&self, i: &ItemStatus) -> String {
        i.uptime_90d
            .map(|u| format!("{u:.2}% uptime"))
            .unwrap_or_else(|| "no data yet".into())
    }
    fn state_word(&self, i: &ItemStatus) -> &'static str {
        match i.state {
            "down" => "Down",
            "issue" => "Issue",
            "unknown" => "Waiting",
            _ => "Up",
        }
    }
    fn when(&self, i: &IncidentStatus) -> String {
        match i.resolved_at {
            Some(r) => format!(
                "{} · resolved after {}",
                time::utc(i.started_at),
                time::duration(r - i.started_at)
            ),
            None => format!("{} · ongoing", time::utc(i.started_at)),
        }
    }
    fn hhmm(&self, ts: &i64) -> String {
        time::utc(*ts)
    }
    fn label(&self, l: &str) -> &'static str {
        crate::incidents::routes::label_title(l)
    }
}

async fn page(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Response, PageError> {
    let status = super::load(&state.db, &slug, DAYS)
        .await?
        .ok_or(PageError::NotFound)?;
    Ok(crate::platform::render::public_cache(render(&Page {
        status,
    })))
}
