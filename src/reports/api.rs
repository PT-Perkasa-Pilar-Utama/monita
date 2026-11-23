//! Report downloads (v1.2 Q59). Contract: docs/api-specs/03-projects.md and 05-incidents.md.

use axum::{
    Router,
    extract::{Path, Query, State},
    http::header,
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Deserialize;

use crate::{
    accounts::CurrentUser,
    app::AppState,
    platform::{now, settings, ui::UiError},
    projects,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/ui/projects/{slug}/report", get(summary))
        .route(
            "/api/ui/projects/{slug}/incidents/{id}/report",
            get(incident),
        )
}

/// `period` is `week`, `7d`, `month` or `quarter`; anything else reads `from` and `to`.
#[derive(Deserialize, Default)]
#[serde(default)]
struct RangeQuery {
    period: String,
    from: String,
    to: String,
}

fn pdf(name: &str, body: Vec<u8>) -> Response {
    (
        [
            (header::CONTENT_TYPE, "application/pdf".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{name}\""),
            ),
        ],
        body,
    )
        .into_response()
}

/// The summary report of a range of days, in the admin's offset.
async fn summary(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
    Query(q): Query<RangeQuery>,
) -> Result<Response, UiError> {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    let off = settings::utc_offset(&state.db.read().await?).await?;
    let t = now();
    let range = super::range_of(&q.period, &q.from, &q.to, t, off)
        .map_err(|e| UiError::field("from", e.to_string()))?;
    let body = super::summary_pdf(&state.db, &p, range, t, off).await?;
    Ok(pdf(&super::file_name(&p.slug, range, off), body))
}

/// The report of one incident.
async fn incident(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, id)): Path<(String, i64)>,
) -> Result<Response, UiError> {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    let off = settings::utc_offset(&state.db.read().await?).await?;
    let body = super::incident_pdf(&state.db, &p, id, now(), off)
        .await?
        .ok_or(UiError::NotFound)?;
    Ok(pdf(&format!("{}-inc-{id}.pdf", p.slug), body))
}
