//! The tech lead's ticket board and ticket detail.

use super::*;

/// The newest tickets of each column, and how many each column holds.
pub(super) async fn board(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(slug): Path<String>,
) -> UiResult {
    let p = projects::find_for(&state.db, &user, &slug).await?;
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
    done(crate::tickets::move_stage(&state.db, p.id, number, &f.stage, reason).await?)
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
