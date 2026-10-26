//! US-20: a manual incident, opened by a TL from the form.

use super::*;

/// A manual incident as the TL fills it in (US-20). `items` are the form's `kind:id` values.
pub struct NewIncident {
    pub project_id: i64,
    pub title: String,
    pub items: Vec<String>,
    pub severity: String,
    pub message: String,
    pub author: Author,
}

/// US-20. A manual incident starts with an Investigating update.
pub async fn open_manual(db: &Db, n: &NewIncident, now: i64) -> Result<i64, UpdateError> {
    if n.title.trim().is_empty() {
        return Err(UpdateError::EmptyTitle);
    }
    if n.title.trim().chars().count() > MAX_TITLE {
        return Err(UpdateError::TitleTooLong);
    }
    // Settled in CLAUDE.md: items are checked on a read connection, so a rejected form never
    // takes the writer, then again inside the write, so a source deleted meanwhile is skipped.
    if picked(&item_names(db, n.project_id).await?, n).is_empty() {
        return Err(UpdateError::NoItems);
    }
    if !matches!(n.severity.as_str(), "critical" | "warning") {
        return Err(UpdateError::BadSeverity);
    }
    check_message(&n.message)?;
    if store::manual_full(&db.read().await?, n.project_id).await? {
        return Err(UpdateError::TooManyOpen);
    }
    db.write(async |c| {
        if store::manual_full(c, n.project_id).await? {
            return Ok(Err(UpdateError::TooManyOpen));
        }
        let items = picked(&item_names_on(c, n.project_id).await?, n);
        if items.is_empty() {
            return Ok(Err(UpdateError::NoItems));
        }
        Ok(Ok(store::open_manual(c, n, &items, now).await?))
    })
    .await?
}

/// The project's items that the form named, once each.
fn picked(names: &[(ItemRef, String)], n: &NewIncident) -> Vec<ItemRef> {
    names
        .iter()
        .filter(|(item, _)| {
            n.items
                .contains(&format!("{}:{}", item.kind.as_str(), item.id))
        })
        .map(|(item, _)| item.clone())
        .collect()
}
