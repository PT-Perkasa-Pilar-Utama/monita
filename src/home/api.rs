//! `/api/ui/incidents`, `/api/ui/tickets` and `/api/ui/home-counts`: the cross-project lists and
//! the numbers beside the top menu. A tech lead sees their own projects; a super admin sees all.

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
        .route("/api/ui/home-counts", get(counts))
}

/// Whose projects a list reads: `None` reads every project.
fn owner(user: &User) -> Option<i64> {
    (!user.is_admin()).then_some(user.id)
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct PageQuery {
    page: String,
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
    let (rows, total) = crate::incidents::open_across(&state.db, owner(&user), q.page()).await?;
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
    let (rows, total) = crate::tickets::triage_across(&state.db, owner(&user), q.page()).await?;
    let rows: Vec<(i64, TriageRow)> = rows.into_iter().map(|r| (r.project_id, r)).collect();
    let rows = with_projects(&state, rows).await?;
    Ok(data(serde_json::json!({
        "tickets": rows, "total": total, "page": q.page(), "page_size": caps::ACROSS_PAGE,
    })))
}

/// The numbers beside Projects, Incidents and Tickets in the top menu.
async fn counts(State(state): State<AppState>, CurrentUser(user): CurrentUser) -> UiResult {
    let o = owner(&user);
    Ok(data(serde_json::json!({
        "projects": projects::page_for(&state.db, &user, "", "", projects::Sort::Name, 1).await?.total,
        "incidents": crate::incidents::open_across(&state.db, o, 1).await?.1,
        "tickets": crate::tickets::triage_across(&state.db, o, 1).await?.1,
    })))
}
