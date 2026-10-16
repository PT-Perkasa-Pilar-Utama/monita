//! Public pages: the report form, the reporter's list, the public board.

use super::*;

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

#[derive(Deserialize, Default)]
#[serde(default)]
pub(super) struct Created {
    created: Option<String>,
}

/// After a submit the form redirects here with `?created=<number>`, so a reload never resubmits.
pub(super) async fn report_form(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    axum::extract::Query(q): axum::extract::Query<Created>,
) -> Result<Response, PageError> {
    let project = public_project(&state, &slug).await?;
    Ok(render(&ReportPage {
        project,
        ref_id: String::new(),
        title: String::new(),
        description: String::new(),
        page_url: String::new(),
        error: String::new(),
        created: q.created.and_then(|n| n.parse().ok()),
    }))
}

#[derive(Deserialize)]
pub(super) struct ReportForm {
    #[serde(default)]
    ref_id: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    page_url: String,
}

pub(super) async fn report_submit(
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
    match crate::tickets::submit(
        &state.db,
        &state.config.base_url,
        Some(page.project.id),
        &f.ref_id,
        &ticket,
    )
    .await
    {
        Ok(c) => Ok(Redirect::to(&format!(
            "/status/{}/report?created={}",
            page.project.slug, c.number
        ))
        .into_response()),
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
        crate::tickets::stage_label(s)
    }
    fn reason(&self, r: &str) -> &'static str {
        crate::tickets::reason_label(r)
    }
    fn utc(&self, ts: &i64) -> String {
        time::utc(*ts)
    }
}

pub(super) async fn mine_form(
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
pub(super) struct MineForm {
    #[serde(default)]
    ref_id: String,
}

pub(super) async fn mine_lookup(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Form(f): Form<MineForm>,
) -> Result<Response, PageError> {
    let project = public_project(&state, &slug).await?;
    match crate::tickets::mine(&state.db, &f.ref_id, Some(project.id)).await {
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
        crate::tickets::reason_label(r)
    }
    fn day(&self, ts: &i64) -> String {
        time::day(*ts)
    }
}

pub(super) async fn public_board(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Response, PageError> {
    let project = public_project(&state, &slug).await?;
    let tickets = crate::tickets::public_board(&state.db, project.id).await?;
    Ok(crate::platform::render::public_cache(render(&BoardPage {
        project,
        tickets,
    })))
}
