//! Monitors and heartbeats as JSON for the frontend. Contract: docs/api-specs/04-checks.md.

mod batch;
mod heartbeats;
mod locations;
mod monitors;
mod pins;
mod views;

use axum::{Router, extract::State, routing::get};
use serde::Deserialize;

use super::chart::Range;
use crate::{
    accounts::CurrentUser,
    app::AppState,
    platform::ui::{UiError, UiResult, data},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/ui/check-options", get(options))
        .nest("/api/ui/projects/{slug}", project_routes())
        .merge(locations::router())
        .merge(pins::router())
}

/// The routes below `/api/ui/projects/{slug}` that a project API key also reaches under
/// `/api/v1/projects/{slug}` (v1.1 Q44).
pub fn project_routes() -> Router<AppState> {
    monitors::project_routes()
        .merge(heartbeats::project_routes())
        .merge(batch::project_routes())
}

/// Field errors from a form's `validate`, as a 422 on those fields.
fn invalid(errors: super::FieldErrors) -> UiError {
    UiError::fields(
        errors
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
    )
}

/// `?before=<time>.<id>&range=hour|day|week|month` on a detail page.
#[derive(Deserialize, Default)]
#[serde(default)]
struct Paging {
    /// A cursor the page got from the server; a hand-edited bad value reads the first page.
    before: Option<String>,
    range: Option<String>,
}

impl Paging {
    /// `<time>.<id>`: two rows can share a second, so the time alone would skip one.
    fn cursor(&self) -> Option<(i64, i64)> {
        let (t, id) = self.before.as_deref()?.split_once('.')?;
        Some((t.parse().ok()?, id.parse().ok()?))
    }

    fn range(&self) -> Range {
        Range::parse(self.range.as_deref())
    }
}

#[derive(serde::Serialize)]
struct Choice {
    value: i64,
    label: String,
}

/// What the monitor and heartbeat forms offer, and their defaults.
async fn options(State(state): State<AppState>, CurrentUser(_): CurrentUser) -> UiResult {
    let choices = |table: &[(i64, &str)]| -> Vec<Choice> {
        table
            .iter()
            .map(|(v, l)| Choice {
                value: *v,
                label: l.to_string(),
            })
            .collect()
    };
    let intervals: Vec<Choice> = super::INTERVALS_S
        .iter()
        .map(|&s| Choice {
            value: s,
            label: super::overview::interval_label(s).to_string(),
        })
        .collect();
    let m = super::MonitorInput::defaults();
    let locations: Vec<serde_json::Value> = super::locations::names(&state.db)
        .await?
        .into_iter()
        .map(|(id, name)| serde_json::json!({ "id": id, "name": name }))
        .collect();
    let h = super::heartbeat::HeartbeatInput::default();
    Ok(data(serde_json::json!({
        "intervals": intervals,
        "periods": choices(&super::heartbeat::PERIODS_S),
        "graces": choices(&super::heartbeat::GRACES_S),
        "fails": {
            "min": crate::incidents::engine::FAILS_RANGE.start(),
            "max": crate::incidents::engine::FAILS_RANGE.end(),
        },
        "monitor": {
            "interval_s": m.interval_s,
            "timeout_s": m.timeout_ms / 1000,
            "expected_status": m.expected_status,
            "severity": m.severity,
        },
        "locations": locations,
        "heartbeat": {
            "period_s": h.period_s,
            "grace_s": h.grace_s,
            "severity": h.severity,
        },
    })))
}

/// The pause switch of a monitor or heartbeat.
#[derive(serde::Deserialize)]
pub struct PausedInput {
    pub paused: bool,
}
