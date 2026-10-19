//! Public JSON for the report form, the reporter's list and the public board. No sign-in; the
//! per-IP limit covers `/api/ui/public/` (SEC-12). SEC-14: no description or reporter leaves here.

use super::*;

async fn public_project(state: &AppState, slug: &str) -> Result<projects::Project, UiError> {
    projects::find_public(&state.db, slug)
        .await?
        .ok_or(UiError::NotFound)
}

/// The project's name for a public page's header.
pub(super) async fn project(State(state): State<AppState>, Path(slug): Path<String>) -> UiResult {
    let p = public_project(&state, &slug).await?;
    let off = crate::platform::settings::utc_offset(&state.db.read().await?).await?;
    Ok(data(
        serde_json::json!({ "name": p.name, "slug": p.slug, "pages": p.pages, "utc_offset_min": off }),
    ))
}

/// US-28. Tickets out of Triage, public fields only.
pub(super) async fn public_board(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(q): Query<ColumnQuery>,
) -> UiResult {
    let p = public_project(&state, &slug).await?;
    if !p.pages.known_issues {
        return Err(UiError::NotFound);
    }
    if let Some(stage) = q.stage()? {
        let page = q.page.unwrap_or(1);
        let tickets = crate::tickets::public_column(&state.db, p.id, stage, page).await?;
        let total = if stage == "triage" {
            0
        } else {
            stage_total(&state, p.id, stage).await?
        };
        let body = serde_json::json!({ "tickets": tickets, "page": page, "total": total, "page_size": crate::platform::caps::TICKETS_PER_COLUMN });
        return Ok(crate::platform::ui::public_cache(data(body)));
    }
    let (mut tickets, rows) = crate::tickets::public_board(&state.db, p.id).await?;
    // SEC-14, second layer: the SQL leaves Triage out, and so does this, count included.
    tickets.retain(|t| t.stage != "triage");
    let mut counts = counts(rows);
    counts.remove("triage");
    let body = serde_json::json!({ "tickets": tickets, "counts": counts });
    Ok(crate::platform::ui::public_cache(data(body)))
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub(super) struct ReportForm {
    ref_id: String,
    title: String,
    description: String,
    page_url: String,
}

/// US-24 from the project's own form: the ref ID must belong to this project. 201 `{number}`.
pub(super) async fn report(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Body(f): Body<ReportForm>,
) -> UiResult {
    let p = public_project(&state, &slug).await?;
    if !p.pages.report {
        return Err(UiError::NotFound);
    }
    let ticket = NewTicket {
        title: f.title,
        description: f.description,
        page_url: Some(f.page_url.trim().to_string()).filter(|u| !u.is_empty()),
    };
    let made = crate::tickets::submit(
        &state.db,
        &state.config.base_url,
        Some(p.id),
        &f.ref_id,
        &ticket,
    )
    .await?;
    Ok(created(serde_json::json!({ "number": made.number })))
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub(super) struct MineForm {
    ref_id: String,
}

/// US-27. The reporter's own tickets in this project. A POST keeps the ref ID out of URLs.
pub(super) async fn mine(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Body(f): Body<MineForm>,
) -> UiResult {
    let p = public_project(&state, &slug).await?;
    if !p.pages.report {
        return Err(UiError::NotFound);
    }
    let tickets = crate::tickets::mine(&state.db, &f.ref_id, Some(p.id)).await?;
    Ok(data(serde_json::json!({ "tickets": tickets })))
}
