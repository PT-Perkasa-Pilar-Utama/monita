//! The tech lead's ticket board and ticket detail.

use super::*;

/// The newest tickets of each column, and how many each column holds. With `?stage=` it is one
/// page of that column.
pub(super) async fn board(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
    Query(q): Query<ColumnQuery>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    if let Some(stage) = q.stage()? {
        let page = q.page.unwrap_or(1);
        let tickets = crate::tickets::column(&state.db, p.id, stage, page).await?;
        let total = stage_total(&state, p.id, stage).await?;
        return Ok(data(
            serde_json::json!({ "tickets": tickets, "page": page, "total": total, "page_size": crate::platform::caps::TICKETS_PER_COLUMN }),
        ));
    }
    let (tickets, rows) = crate::tickets::board(&state.db, p.id).await?;
    Ok(data(
        serde_json::json!({ "tickets": tickets, "counts": counts(rows) }),
    ))
}

pub(super) async fn detail(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, number)): Path<(String, i64)>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    let ticket = crate::tickets::find(&state.db, p.id, number)
        .await?
        .ok_or(UiError::NotFound)?;
    Ok(data(ticket))
}

/// 204 when the ticket changed; an unknown ticket is 404.
fn done(changed: bool) -> UiResult {
    if changed {
        Ok(StatusCode::NO_CONTENT.into_response())
    } else {
        Err(UiError::NotFound)
    }
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub(super) struct StageForm {
    stage: String,
    close_reason: Option<String>,
}

/// US-25. Done needs a close reason; other columns drop it.
pub(super) async fn move_stage(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, number)): Path<(String, i64)>,
    Body(f): Body<StageForm>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    let reason = f.close_reason.as_deref().filter(|r| !r.is_empty());
    let base = &state.config.base_url;
    done(crate::tickets::move_stage(&state.db, base, p.id, number, &f.stage, reason).await?)
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub(super) struct TitleForm {
    public_title: String,
}

/// US-26. A blank title clears it.
pub(super) async fn public_title(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, number)): Path<(String, i64)>,
    Body(f): Body<TitleForm>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    done(crate::tickets::set_public_title(&state.db, p.id, number, &f.public_title).await?)
}

/// US-29. The reporter's ref ID stops working.
pub(super) async fn revoke_ref(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((slug, number)): Path<(String, i64)>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    done(crate::tickets::revoke_ref(&state.db, p.id, number).await?)
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub(super) struct RefIdForm {
    single_use: bool,
}

/// US-23 from the board: the team issues a ref ID for someone who reports outside the app.
/// 201 with the raw ref ID, shown once; only its hash is stored.
pub(super) async fn issue_ref_id(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
    Body(f): Body<RefIdForm>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    let (ref_id, created_at) = crate::tickets::issue_ref_id(&state.db, p.id, f.single_use).await?;
    Ok(crate::platform::ui::created(serde_json::json!({
        "ref_id": ref_id,
        "single_use": f.single_use,
        "created_at": created_at,
    })))
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub(super) struct FileForm {
    title: String,
    description: String,
    stage: String,
}

/// The team files a ticket itself. 201 `{number}`.
pub(super) async fn file(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
    Body(f): Body<FileForm>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
    let t = NewTicket {
        title: f.title,
        description: f.description,
        page_url: None,
    };
    let number = crate::tickets::file(&state.db, p.id, user.id, &f.stage, &t).await?;
    Ok(created(serde_json::json!({ "number": number })))
}
