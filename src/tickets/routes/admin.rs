//! The TL's kanban and ticket detail.

use super::*;

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
        crate::tickets::reason_label(r)
    }
}

pub(super) async fn kanban(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
) -> Result<Response, PageError> {
    let project = projects::find_for(&state.db, &user, &slug).await?;
    let tickets = crate::tickets::board(&state.db, project.id).await?;
    Ok(render(&KanbanPage {
        shell: projects::shell(&user, "tickets", Some(&project)),
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
    let ticket = crate::tickets::find(&state.db, project.id, number)
        .await?
        .ok_or(PageError::NotFound)?;
    Ok(DetailPage {
        shell: projects::shell(user, "tickets", Some(&project)),
        project,
        ticket,
        error: String::new(),
    })
}

pub(super) async fn detail(
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
pub(super) struct StageForm {
    stage: String,
    close_reason: Option<String>,
}

pub(super) async fn move_stage(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, number)): Path<(String, i64)>,
    Form(f): Form<StageForm>,
) -> Result<Response, PageError> {
    let project = projects::find_for(&state.db, &user, &slug).await?;
    let result = crate::tickets::move_stage(
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
pub(super) struct TitleForm {
    #[serde(default)]
    public_title: String,
}

pub(super) async fn public_title(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, number)): Path<(String, i64)>,
    Form(f): Form<TitleForm>,
) -> Result<Response, PageError> {
    let project = projects::find_for(&state.db, &user, &slug).await?;
    let result =
        crate::tickets::set_public_title(&state.db, project.id, number, &f.public_title).await;
    after_edit(&state, &user, &slug, number, result).await
}

pub(super) async fn revoke_ref(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, number)): Path<(String, i64)>,
) -> Result<Response, PageError> {
    let project = projects::find_for(&state.db, &user, &slug).await?;
    let result = crate::tickets::revoke_ref(&state.db, project.id, number)
        .await
        .map_err(TicketError::Other);
    after_edit(&state, &user, &slug, number, result).await
}
