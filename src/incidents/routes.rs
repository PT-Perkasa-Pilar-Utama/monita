//! Incident pages. Contract: docs/api-specs/05-incidents.md.

use askama::Template;
use axum::{
    Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use serde::Deserialize;

use super::{IncidentRow, UpdateError, UpdateRow};
use crate::{
    accounts::CurrentUser,
    app::AppState,
    platform::{
        now,
        page::{PageError, Shell},
        render::render,
        time,
    },
    projects::{self, Project},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/projects/{slug}/incidents", get(list).post(create))
        .route("/projects/{slug}/incidents/new", get(new_form))
        .route("/projects/{slug}/incidents/{id}", get(detail))
        .route("/projects/{slug}/incidents/{id}/updates", post(post_update))
}

#[derive(Template)]
#[template(path = "incidents/list.html")]
struct ListPage {
    shell: Shell,
    project: Project,
    open: Vec<IncidentRow>,
    resolved: Vec<IncidentRow>,
    show_resolved: bool,
    now: i64,
}

impl ListPage {
    fn rows(&self) -> &[IncidentRow] {
        if self.show_resolved {
            &self.resolved
        } else {
            &self.open
        }
    }
    fn duration(&self, i: &IncidentRow) -> String {
        super::duration(i.started_at, i.resolved_at, self.now)
    }
    fn started(&self, i: &IncidentRow) -> String {
        time::utc(i.started_at)
    }
    fn label(&self, i: &IncidentRow) -> &'static str {
        super::label_title(i.last_label.as_deref().unwrap_or(""))
    }
}

#[derive(Deserialize)]
struct ListQuery {
    state: Option<String>,
}

async fn list(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
    Query(q): Query<ListQuery>,
) -> Result<Response, PageError> {
    let project = projects::find_for(&state.db, &user, &slug).await?;
    let show_resolved = q.state.as_deref() == Some("resolved");
    let open = super::list(&state.db, project.id, true).await?;
    let resolved = if show_resolved {
        super::list(&state.db, project.id, false).await?
    } else {
        Vec::new()
    };
    Ok(render(&ListPage {
        shell: projects::shell(&user, "incidents", Some(&project)),
        project,
        open,
        resolved,
        show_resolved,
        now: now(),
    }))
}

#[derive(Template)]
#[template(path = "incidents/detail.html")]
struct DetailPage {
    shell: Shell,
    project: Project,
    incident: IncidentRow,
    items: Vec<String>,
    updates: Vec<UpdateRow>,
    label: String,
    message: String,
    error: String,
    now: i64,
}

impl DetailPage {
    fn duration(&self) -> String {
        super::duration(
            self.incident.started_at,
            self.incident.resolved_at,
            self.now,
        )
    }
    fn utc(&self, ts: &i64) -> String {
        time::utc(*ts)
    }
    fn hhmm(&self, ts: &i64) -> String {
        time::utc(*ts)[11..16].to_string()
    }
    fn title(&self, label: &str) -> &'static str {
        super::label_title(label)
    }
    fn labels(&self) -> &'static [&'static str] {
        super::allowed_labels(&self.incident.origin, self.incident.resolved_at.is_some())
    }
    fn dot(&self, label: &str) -> &'static str {
        match label {
            "investigating" => "down",
            "identified" => "progress",
            "monitoring" => "done",
            _ => "up",
        }
    }
}

async fn detail_page(
    state: &AppState,
    user: &crate::accounts::User,
    project: Project,
    id: i64,
) -> Result<DetailPage, PageError> {
    let (incident, items, updates) = super::find(&state.db, project.id, id)
        .await?
        .ok_or(PageError::NotFound)?;
    let all = super::item_names(&state.db, project.id).await?;
    let items = items
        .iter()
        .map(|it| {
            all.iter()
                .find(|(r, _)| r == it)
                .map(|(_, n)| n.clone())
                .unwrap_or_else(|| format!("Deleted {}", it.kind.as_str()))
        })
        .collect();
    let label = if incident.resolved_at.is_some() {
        "resolved"
    } else {
        "monitoring"
    }
    .to_string();
    Ok(DetailPage {
        shell: projects::shell(user, "incidents", Some(&project)),
        project,
        incident,
        items,
        updates,
        label,
        message: String::new(),
        error: String::new(),
        now: now(),
    })
}

async fn detail(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, id)): Path<(String, i64)>,
) -> Result<Response, PageError> {
    let project = projects::find_for(&state.db, &user, &slug).await?;
    Ok(render(&detail_page(&state, &user, project, id).await?))
}

#[derive(Deserialize)]
struct UpdateForm {
    label: String,
    #[serde(default)]
    message: String,
}

async fn post_update(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, id)): Path<(String, i64)>,
    axum::Form(f): axum::Form<UpdateForm>,
) -> Result<Response, PageError> {
    let project = projects::find_for(&state.db, &user, &slug).await?;
    match super::post_update(
        &state.db,
        project.id,
        id,
        &f.label,
        &f.message,
        user.id,
        now(),
    )
    .await
    {
        Ok(true) => Ok(Redirect::to(&format!("/projects/{slug}/incidents/{id}")).into_response()),
        Ok(false) => Err(PageError::NotFound),
        Err(UpdateError::Other(e)) => Err(e.into()),
        Err(e) => {
            let mut page = detail_page(&state, &user, project, id).await?;
            page.label = f.label;
            page.message = f.message;
            page.error = e.to_string();
            Ok((StatusCode::UNPROCESSABLE_ENTITY, render(&page)).into_response())
        }
    }
}

#[derive(Template)]
#[template(path = "incidents/new.html")]
struct NewPage {
    shell: Shell,
    project: Project,
    choices: Vec<(String, String, bool)>,
    title: String,
    message: String,
    severity: String,
    error: String,
}

async fn new_page(
    state: &AppState,
    user: &crate::accounts::User,
    project: Project,
    picked: &[String],
) -> Result<NewPage, PageError> {
    let choices = super::item_names(&state.db, project.id)
        .await?
        .into_iter()
        .map(|(r, n)| {
            let value = format!("{}:{}", r.kind.as_str(), r.id);
            let checked = picked.contains(&value);
            (value, n, checked)
        })
        .collect();
    Ok(NewPage {
        shell: projects::shell(user, "incidents", Some(&project)),
        project,
        choices,
        title: String::new(),
        message: String::new(),
        severity: "warning".into(),
        error: String::new(),
    })
}

async fn new_form(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
) -> Result<Response, PageError> {
    let project = projects::find_for(&state.db, &user, &slug).await?;
    Ok(render(&new_page(&state, &user, project, &[]).await?))
}

/// `items` repeats in the form (one per checkbox), so the body is read as pairs.
async fn create(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
    axum::Form(pairs): axum::Form<Vec<(String, String)>>,
) -> Result<Response, PageError> {
    let project = projects::find_for(&state.db, &user, &slug).await?;
    let field = |k: &str| {
        pairs
            .iter()
            .find(|(key, _)| key == k)
            .map(|(_, v)| v.clone())
            .unwrap_or_default()
    };
    let picked: Vec<String> = pairs
        .iter()
        .filter(|(k, _)| k == "items")
        .map(|(_, v)| v.clone())
        .collect();
    let incident = super::NewIncident {
        project_id: project.id,
        title: field("title"),
        items: picked.clone(),
        severity: field("severity"),
        message: field("message"),
        author: user.id,
    };
    match super::open_manual(&state.db, &incident, now()).await {
        Ok(id) => Ok(Redirect::to(&format!("/projects/{slug}/incidents/{id}")).into_response()),
        Err(UpdateError::Other(e)) => Err(e.into()),
        Err(e) => {
            let mut page = new_page(&state, &user, project, &picked).await?;
            page.title = field("title");
            page.message = field("message");
            page.severity = incident.severity;
            page.error = e.to_string();
            Ok((StatusCode::UNPROCESSABLE_ENTITY, render(&page)).into_response())
        }
    }
}
