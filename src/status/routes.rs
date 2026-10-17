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

use super::{Day, IncidentStatus, ItemStatus, ProjectRef, Status};
use crate::{
    app::AppState,
    platform::{api_error, caps, page::PageError, render::render, time},
};

const DAYS: usize = 90;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/status/{slug}", get(page))
        .route("/status/{slug}/history", get(history))
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

/// Incident wording shared by the page and the history page.
struct Words;

impl Words {
    fn when(&self, i: &IncidentStatus) -> String {
        match i.resolved_at {
            Some(r) => format!(
                "{}, resolved after {}",
                time::utc(i.started_at),
                time::duration(r - i.started_at)
            ),
            None => format!("Since {}", time::utc(i.started_at)),
        }
    }
    fn utc(&self, ts: &i64) -> String {
        time::utc(*ts)
    }
    fn label(&self, l: &str) -> &'static str {
        crate::incidents::label_title(l)
    }
    fn tone(&self, l: &str) -> &'static str {
        match l {
            "resolved" => "up",
            "investigating" => "down",
            "identified" => "issue",
            _ => "unknown",
        }
    }
}

#[derive(Template)]
#[template(path = "status/page.html")]
struct Page {
    status: Status,
    recent: Vec<IncidentStatus>,
    words: Words,
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
    fn up_count(&self) -> usize {
        self.status.items.iter().filter(|i| i.state == "up").count()
    }
    /// Down and issue first, with their bars.
    fn problems(&self) -> Vec<&ItemStatus> {
        let rank = |s: &str| match s {
            "down" => 0,
            "issue" => 1,
            _ => 2,
        };
        let mut v: Vec<&ItemStatus> = self
            .status
            .items
            .iter()
            .filter(|i| rank(i.state) < 2)
            .collect();
        v.sort_by_key(|i| rank(i.state));
        v
    }
    fn calm(&self) -> Vec<&ItemStatus> {
        self.status
            .items
            .iter()
            .filter(|i| !matches!(i.state, "down" | "issue"))
            .collect()
    }
    /// The first operational systems keep their bars; the rest are a plain list.
    fn barred(&self) -> Vec<&ItemStatus> {
        self.calm().into_iter().take(caps::PUBLIC_BARS).collect()
    }
    fn rest(&self) -> Vec<&ItemStatus> {
        self.calm().into_iter().skip(caps::PUBLIC_BARS).collect()
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
}

async fn page(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Response, PageError> {
    let data = super::page(&state.db, &slug)
        .await?
        .ok_or(PageError::NotFound)?;
    Ok(crate::platform::render::public_cache(render(&Page {
        status: data.status,
        recent: data.recent,
        words: Words,
    })))
}

#[derive(Template)]
#[template(path = "status/history.html")]
struct History {
    project: ProjectRef,
    incidents: Vec<IncidentStatus>,
    older: Option<String>,
    words: Words,
}

impl History {
    /// "October 2026" above the first incident of each month.
    fn month(&self, i: &usize) -> Option<String> {
        let i = *i;
        const NAMES: [&str; 12] = [
            "January",
            "February",
            "March",
            "April",
            "May",
            "June",
            "July",
            "August",
            "September",
            "October",
            "November",
            "December",
        ];
        let key = |n: usize| time::day(self.incidents[n].started_at)[..7].to_string();
        let this = key(i);
        if i > 0 && key(i - 1) == this {
            return None;
        }
        let month: usize = this[5..7].parse().ok()?;
        Some(format!("{} {}", NAMES[month - 1], &this[..4]))
    }
}

#[derive(Deserialize)]
struct Before {
    /// `<started_at>.<id>`, a cursor the page links to; a bad value shows the first page.
    before: Option<String>,
}

async fn history(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(q): Query<Before>,
) -> Result<Response, PageError> {
    let cursor = q.before.as_deref().and_then(|b| {
        let (t, id) = b.split_once('.')?;
        Some((t.parse().ok()?, id.parse().ok()?))
    });
    let (project, incidents) = super::history_page(&state.db, &slug, cursor)
        .await?
        .ok_or(PageError::NotFound)?;
    let older = (incidents.len() as i64 == caps::PUBLIC_HISTORY_PAGE)
        .then(|| {
            incidents
                .last()
                .map(|i| format!("{}.{}", i.started_at, i.id))
        })
        .flatten();
    Ok(crate::platform::render::public_cache(render(&History {
        project,
        incidents,
        older,
        words: Words,
    })))
}
