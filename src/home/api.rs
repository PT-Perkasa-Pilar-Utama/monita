//! `/api/ui/incidents` and `/api/ui/tickets`: the cross-project lists. A tech lead sees their own projects; a super admin sees all.

use std::collections::HashMap;

use axum::{
    Router,
    extract::{Query, State},
    routing::get,
};
use serde::{Deserialize, Serialize};

use crate::{
    accounts::{CurrentUser, User},
    app::AppState,
    incidents::IncidentRow,
    platform::{
        caps,
        ui::{UiResult, data},
    },
    projects::{self, Project, api::ProjectView},
    tickets::TriageRow,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/ui/incidents", get(incidents))
        .route("/api/ui/tickets", get(tickets))
}

/// Whose projects a list reads: `None` reads every project.
fn owner(user: &User) -> Option<i64> {
    (!user.is_admin()).then_some(user.id)
}

/// The list's URL: `?q=&state=&severity=&sort=&page=`. Unknown values read as the default.
#[derive(Deserialize, Default)]
#[serde(default)]
struct PageQuery {
    page: String,
    q: String,
    state: String,
    severity: String,
    sort: String,
}

impl PageQuery {
    fn page(&self) -> i64 {
        self.page.trim().parse().unwrap_or(1).max(1)
    }
}

/// A row with the project it belongs to.
#[derive(Serialize)]
struct WithProject<T: Serialize> {
    project: ProjectView,
    #[serde(flatten)]
    row: T,
}

async fn with_projects<T: Serialize>(
    state: &AppState,
    rows: Vec<(i64, T)>,
) -> anyhow::Result<Vec<WithProject<T>>> {
    let ids: Vec<i64> = rows.iter().map(|(id, _)| *id).collect();
    let found: HashMap<i64, Project> = projects::by_ids(&state.db, &ids).await?;
    Ok(rows
        .into_iter()
        .filter_map(|(id, row)| {
            found.get(&id).map(|p| WithProject {
                project: ProjectView::from(p),
                row,
            })
        })
        .collect())
}

async fn incidents(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Query(q): Query<PageQuery>,
) -> UiResult {
    let f = crate::incidents::AcrossFilter {
        q: &q.q,
        state: &q.state,
        severity: &q.severity,
        sort: &q.sort,
    };
    let (rows, total) = crate::incidents::across(&state.db, owner(&user), &f, q.page()).await?;
    let rows: Vec<WithProject<IncidentRow>> = with_projects(&state, rows).await?;
    Ok(data(serde_json::json!({
        "incidents": rows, "total": total, "page": q.page(), "page_size": caps::ACROSS_PAGE,
    })))
}

async fn tickets(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Query(q): Query<PageQuery>,
) -> UiResult {
    let (rows, total) =
        crate::tickets::triage_across(&state.db, owner(&user), &q.q, &q.sort, q.page()).await?;
    let rows: Vec<(i64, TriageRow)> = rows.into_iter().map(|r| (r.project_id, r)).collect();
    let rows = with_projects(&state, rows).await?;
    Ok(data(serde_json::json!({
        "tickets": rows, "total": total, "page": q.page(), "page_size": caps::ACROSS_PAGE,
    })))
}
