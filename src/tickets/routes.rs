//! Ticket API and pages. Contract: docs/api-specs/08-tickets.md.

use askama::Template;
use axum::{
    Form, Json, Router,
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use serde::Deserialize;

use super::{CLOSE_REASONS, MyTicket, NewTicket, PublicTicket, STAGES, TicketError, TicketRow};
use crate::{
    accounts::CurrentUser,
    app::AppState,
    platform::{
        api_error,
        page::{PageError, Shell},
        render::render,
        time,
    },
    projects::{self, ApiProject, Project},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/ref-ids", post(issue_ref_id))
        .route("/api/v1/tickets", post(api_submit).options(preflight))
        .route("/api/v1/tickets/mine", get(api_mine).options(preflight))
        .route(
            "/status/{slug}/report",
            get(report_form).post(report_submit),
        )
        .route("/status/{slug}/mine", get(mine_form).post(mine_lookup))
        .route("/status/{slug}/tickets", get(public_board))
        .route("/projects/{slug}/tickets", get(kanban))
        .route("/projects/{slug}/tickets/{number}", get(detail))
        .route("/projects/{slug}/tickets/{number}/stage", post(move_stage))
        .route(
            "/projects/{slug}/tickets/{number}/public-title",
            post(public_title),
        )
        .route(
            "/projects/{slug}/tickets/{number}/revoke-ref",
            post(revoke_ref),
        )
}

/// Project pages and the reporter's own UI call the ticket API from browsers (api-specs 01 1.8).
fn cors(mut res: Response) -> Response {
    let h = res.headers_mut();
    h.insert(
        header::ACCESS_CONTROL_ALLOW_ORIGIN,
        HeaderValue::from_static("*"),
    );
    h.insert(
        header::ACCESS_CONTROL_ALLOW_METHODS,
        HeaderValue::from_static("GET, POST, OPTIONS"),
    );
    h.insert(
        header::ACCESS_CONTROL_ALLOW_HEADERS,
        HeaderValue::from_static("content-type, x-ref-id"),
    );
    h.insert(
        header::ACCESS_CONTROL_MAX_AGE,
        HeaderValue::from_static("86400"),
    );
    res
}

async fn preflight() -> Response {
    cors(StatusCode::NO_CONTENT.into_response())
}

fn ticket_error(e: TicketError) -> Response {
    match e {
        TicketError::Invalid(m) => api_error(StatusCode::BAD_REQUEST, "invalid_request", m),
        TicketError::RefIdInvalid => api_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "ref_id_invalid",
            &e.to_string(),
        ),
        TicketError::DailyCap => api_error(
            StatusCode::TOO_MANY_REQUESTS,
            "daily_cap_reached",
            &e.to_string(),
        ),
        TicketError::NeedReason => {
            api_error(StatusCode::BAD_REQUEST, "invalid_request", &e.to_string())
        }
        TicketError::Other(e) => {
            tracing::error!(error = %e, "ticket api failed");
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal",
                "Unexpected failure.",
            )
        }
    }
}

#[derive(Deserialize, Default)]
struct RefIdRequest {
    #[serde(default)]
    single_use: bool,
}

async fn issue_ref_id(
    State(state): State<AppState>,
    ApiProject(project): ApiProject,
    body: Bytes,
) -> Response {
    // The body is optional: no body, `null` and `{}` all mean the defaults.
    let req = if body.iter().all(u8::is_ascii_whitespace) {
        RefIdRequest::default()
    } else {
        match serde_json::from_slice::<Option<RefIdRequest>>(&body) {
            Ok(r) => r.unwrap_or_default(),
            Err(_) => {
                return api_error(
                    StatusCode::BAD_REQUEST,
                    "invalid_request",
                    "Body must be JSON like {\"single_use\": true}.",
                );
            }
        }
    };
    match super::issue_ref_id(&state.db, project.id, req.single_use).await {
        Ok((ref_id, created_at)) => (
            StatusCode::CREATED,
            Json(serde_json::json!({ "data": { "ref_id": ref_id, "single_use": req.single_use, "created_at": created_at } })),
        )
            .into_response(),
        Err(e) => ticket_error(TicketError::Other(e)),
    }
}

#[derive(Deserialize)]
struct TicketBody {
    ref_id: String,
    title: String,
    description: String,
    page_url: Option<String>,
}

impl TicketBody {
    fn ticket(&self) -> NewTicket {
        NewTicket {
            title: self.title.clone(),
            description: self.description.clone(),
            page_url: self
                .page_url
                .clone()
                .map(|u| u.trim().to_string())
                .filter(|u| !u.is_empty()),
        }
    }
}

async fn api_submit(State(state): State<AppState>, body: Bytes) -> Response {
    let Ok(req) = serde_json::from_slice::<TicketBody>(&body) else {
        return cors(api_error(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "Body must be JSON with ref_id, title and description.",
        ));
    };
    cors(match super::submit(&state.db, &state.config.base_url, None, &req.ref_id, &req.ticket()).await {
        Ok(c) => (StatusCode::CREATED, Json(serde_json::json!({ "data": { "number": c.number, "stage": "triage", "created_at": c.created_at } }))).into_response(),
        Err(e) => ticket_error(e),
    })
}

async fn api_mine(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let Some(ref_id) = headers
        .get("x-ref-id")
        .and_then(|v| v.to_str().ok())
        .filter(|v| !v.is_empty())
    else {
        return cors(api_error(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "Send the ref ID in the X-Ref-Id header.",
        ));
    };
    cors(match super::mine(&state.db, ref_id, None).await {
        Ok(list) => Json(serde_json::json!({ "data": list, "next_cursor": null })).into_response(),
        Err(e) => ticket_error(e),
    })
}

async fn public_project(state: &AppState, slug: &str) -> Result<Project, PageError> {
    projects::find_public(&state.db, slug)
        .await?
        .ok_or(PageError::NotFound)
}

#[derive(Template)]
#[template(path = "tickets/report.html")]
struct ReportPage {
    project: Project,
    ref_id: String,
    title: String,
    description: String,
    page_url: String,
    error: String,
    created: Option<i64>,
}

impl ReportPage {
    fn ref_help(&self) -> String {
        format!("Your account page in {} shows it.", self.project.name)
    }
    fn thanks(&self, n: &i64) -> String {
        format!(
            "Thanks. Your report is ticket #{n}. Track it with your ref ID on the My tickets page."
        )
    }
}

async fn report_form(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Response, PageError> {
    let project = public_project(&state, &slug).await?;
    Ok(render(&ReportPage {
        project,
        ref_id: String::new(),
        title: String::new(),
        description: String::new(),
        page_url: String::new(),
        error: String::new(),
        created: None,
    }))
}

#[derive(Deserialize)]
struct ReportForm {
    #[serde(default)]
    ref_id: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    page_url: String,
}

async fn report_submit(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Form(f): Form<ReportForm>,
) -> Result<Response, PageError> {
    let project = public_project(&state, &slug).await?;
    let ticket = NewTicket {
        title: f.title.clone(),
        description: f.description.clone(),
        page_url: Some(f.page_url.trim().to_string()).filter(|u| !u.is_empty()),
    };
    let mut page = ReportPage {
        ref_id: f.ref_id.clone(),
        title: f.title,
        description: f.description,
        page_url: f.page_url,
        error: String::new(),
        created: None,
        project,
    };
    match super::submit(
        &state.db,
        &state.config.base_url,
        Some(page.project.id),
        &f.ref_id,
        &ticket,
    )
    .await
    {
        Ok(c) => {
            page.created = Some(c.number);
            Ok(render(&page))
        }
        Err(TicketError::Other(e)) => Err(e.into()),
        Err(e) => {
            let status = if matches!(e, TicketError::DailyCap) {
                StatusCode::TOO_MANY_REQUESTS
            } else {
                StatusCode::UNPROCESSABLE_ENTITY
            };
            page.error = e.to_string();
            Ok((status, render(&page)).into_response())
        }
    }
}

#[derive(Template)]
#[template(path = "tickets/mine.html")]
struct MinePage {
    project: Project,
    tickets: Option<Vec<MyTicket>>,
    error: String,
}

impl MinePage {
    fn stage(&self, s: &str) -> &'static str {
        super::stage_label(s)
    }
    fn reason(&self, r: &str) -> &'static str {
        super::reason_label(r)
    }
    fn utc(&self, ts: &i64) -> String {
        time::utc(*ts)
    }
}

async fn mine_form(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Response, PageError> {
    let project = public_project(&state, &slug).await?;
    Ok(render(&MinePage {
        project,
        tickets: None,
        error: String::new(),
    }))
}

#[derive(Deserialize)]
struct MineForm {
    #[serde(default)]
    ref_id: String,
}

async fn mine_lookup(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Form(f): Form<MineForm>,
) -> Result<Response, PageError> {
    let project = public_project(&state, &slug).await?;
    match super::mine(&state.db, &f.ref_id, Some(project.id)).await {
        Ok(list) => Ok(render(&MinePage {
            project,
            tickets: Some(list),
            error: String::new(),
        })),
        Err(TicketError::Other(e)) => Err(e.into()),
        Err(e) => Ok((
            StatusCode::UNPROCESSABLE_ENTITY,
            render(&MinePage {
                project,
                tickets: None,
                error: e.to_string(),
            }),
        )
            .into_response()),
    }
}

#[derive(Template)]
#[template(path = "tickets/board.html")]
struct BoardPage {
    project: Project,
    tickets: Vec<PublicTicket>,
}

impl BoardPage {
    fn ring(&self, stage: &str) -> &'static str {
        if stage == "backlog" {
            "backlog"
        } else if stage == "in_progress" {
            "progress"
        } else {
            "done"
        }
    }
    fn column(&self, stage: &str) -> Vec<&PublicTicket> {
        self.tickets.iter().filter(|t| t.stage == stage).collect()
    }
    fn columns(&self) -> [(&'static str, &'static str); 3] {
        [STAGES[1], STAGES[2], STAGES[3]]
    }
    fn reason(&self, r: &str) -> &'static str {
        super::reason_label(r)
    }
    fn day(&self, ts: &i64) -> String {
        time::day(*ts)
    }
}

async fn public_board(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Response, PageError> {
    let project = public_project(&state, &slug).await?;
    let tickets = super::public_board(&state.db, project.id).await?;
    let mut res = render(&BoardPage { project, tickets });
    res.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=30"),
    );
    Ok(res)
}

#[derive(Template)]
#[template(path = "tickets/kanban.html")]
struct KanbanPage {
    shell: Shell,
    project: Project,
    tickets: Vec<TicketRow>,
    now: i64,
}

impl KanbanPage {
    fn column(&self, stage: &str) -> Vec<&TicketRow> {
        self.tickets.iter().filter(|t| t.stage == stage).collect()
    }
    fn columns(&self) -> [(&'static str, &'static str); 4] {
        STAGES
    }
    fn ring(&self, stage: &str) -> &'static str {
        match stage {
            "triage" => "triage",
            "backlog" => "backlog",
            "in_progress" => "progress",
            _ => "done",
        }
    }
    fn ago(&self, ts: &i64) -> String {
        time::ago(*ts, self.now)
    }
    fn reason(&self, r: &str) -> &'static str {
        super::reason_label(r)
    }
}

async fn kanban(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
) -> Result<Response, PageError> {
    let project = projects::find_for(&state.db, &user, &slug).await?;
    let tickets = super::board(&state.db, project.id).await?;
    Ok(render(&KanbanPage {
        shell: projects::routes::shell(&user, "tickets", Some(&project)),
        project,
        tickets,
        now: crate::platform::now(),
    }))
}

#[derive(Template)]
#[template(path = "tickets/detail.html")]
struct DetailPage {
    shell: Shell,
    project: Project,
    ticket: TicketRow,
    error: String,
}

impl DetailPage {
    fn stages(&self) -> [(&'static str, &'static str); 4] {
        STAGES
    }
    fn reasons(&self) -> [(&'static str, &'static str); 3] {
        CLOSE_REASONS
    }
    fn utc(&self, ts: &i64) -> String {
        time::utc(*ts)
    }
    /// The title the public board shows once the ticket leaves Triage.
    fn public_shown(&self) -> &str {
        self.ticket
            .public_title
            .as_deref()
            .unwrap_or(&self.ticket.title)
    }
}

async fn detail_page(
    state: &AppState,
    user: &crate::accounts::User,
    slug: &str,
    number: i64,
) -> Result<DetailPage, PageError> {
    let project = projects::find_for(&state.db, user, slug).await?;
    let ticket = super::find(&state.db, project.id, number)
        .await?
        .ok_or(PageError::NotFound)?;
    Ok(DetailPage {
        shell: projects::routes::shell(user, "tickets", Some(&project)),
        project,
        ticket,
        error: String::new(),
    })
}

async fn detail(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, number)): Path<(String, i64)>,
) -> Result<Response, PageError> {
    Ok(render(&detail_page(&state, &user, &slug, number).await?))
}

async fn after_edit(
    state: &AppState,
    user: &crate::accounts::User,
    slug: &str,
    number: i64,
    result: Result<bool, TicketError>,
) -> Result<Response, PageError> {
    match result {
        Ok(true) => Ok(Redirect::to(&format!("/projects/{slug}/tickets/{number}")).into_response()),
        Ok(false) => Err(PageError::NotFound),
        Err(TicketError::Other(e)) => Err(e.into()),
        Err(e) => {
            let mut page = detail_page(state, user, slug, number).await?;
            page.error = e.to_string();
            Ok((StatusCode::UNPROCESSABLE_ENTITY, render(&page)).into_response())
        }
    }
}

#[derive(Deserialize)]
struct StageForm {
    stage: String,
    close_reason: Option<String>,
}

async fn move_stage(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, number)): Path<(String, i64)>,
    Form(f): Form<StageForm>,
) -> Result<Response, PageError> {
    let project = projects::find_for(&state.db, &user, &slug).await?;
    let result = super::move_stage(
        &state.db,
        project.id,
        number,
        &f.stage,
        f.close_reason.as_deref().filter(|r| !r.is_empty()),
    )
    .await;
    after_edit(&state, &user, &slug, number, result).await
}

#[derive(Deserialize)]
struct TitleForm {
    #[serde(default)]
    public_title: String,
}

async fn public_title(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, number)): Path<(String, i64)>,
    Form(f): Form<TitleForm>,
) -> Result<Response, PageError> {
    let project = projects::find_for(&state.db, &user, &slug).await?;
    let result = super::set_public_title(&state.db, project.id, number, &f.public_title).await;
    after_edit(&state, &user, &slug, number, result).await
}

async fn revoke_ref(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, number)): Path<(String, i64)>,
) -> Result<Response, PageError> {
    let project = projects::find_for(&state.db, &user, &slug).await?;
    let result = super::revoke_ref(&state.db, project.id, number)
        .await
        .map_err(TicketError::Other);
    after_edit(&state, &user, &slug, number, result).await
}
