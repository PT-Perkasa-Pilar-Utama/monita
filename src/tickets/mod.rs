//! tickets module: ref IDs, tickets, kanban, public board, the reporter's view.
//! Spec: docs/technical-specs/05-module-definitions.md section 5.7, api-specs 08.

pub mod routes;
mod store;

use crate::platform::{db::Db, now, settings, token};

pub use store::{MyTicket, PublicTicket, TicketRow};

/// The board's columns in order. The page names them (web/src/lib/tickets.ts).
pub const STAGES: [&str; 4] = ["triage", "backlog", "in_progress", "done"];
pub const CLOSE_REASONS: [&str; 3] = ["fixed", "wont_fix", "duplicate"];

/// Tickets not yet done, for the project's tab bar.
pub async fn open_count(db: &Db, project_id: i64) -> anyhow::Result<i64> {
    store::open_count(&db.read().await?, project_id).await
}

pub use store::TriageRow;

/// One page of new tickets in Triage across every project (`owner` None) or the projects `owner`
/// owns, matching `q`, newest first unless `sort` is `oldest`; and how many match in all.
pub async fn triage_across(
    db: &Db,
    owner: Option<i64>,
    q: &str,
    sort: &str,
    page: i64,
) -> anyhow::Result<(Vec<TriageRow>, i64)> {
    let c = db.read().await?;
    let offset = (page.max(1) - 1).saturating_mul(crate::platform::caps::ACROSS_PAGE);
    Ok((
        store::triage_across(
            &c,
            owner,
            q,
            sort,
            crate::platform::caps::ACROSS_PAGE,
            offset,
        )
        .await?,
        store::count_triage_across(&c, owner, q).await?,
    ))
}

/// How many tickets sit in each stage; a stage with none is missing.
pub async fn stage_counts(db: &Db, project_id: i64) -> anyhow::Result<Vec<(String, i64)>> {
    store::stage_counts(&db.read().await?, project_id).await
}

#[derive(Debug, thiserror::Error)]
pub enum TicketError {
    /// Bad values in the form or body: every (field, message), in the form's order.
    #[error("{}", .0.first().map_or("Fix the fields marked below.", |f| f.1))]
    Invalid(Vec<(&'static str, &'static str)>),
    #[error("This ref ID is unknown, revoked or already used.")]
    RefIdInvalid,
    #[error("The daily ticket limit is reached, so try again tomorrow.")]
    DailyCap,
    #[error("Pick a close reason.")]
    NeedReason,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// US-23: returns the raw ref ID once. Only its hash is stored.
pub async fn issue_ref_id(
    db: &Db,
    project_id: i64,
    single_use: bool,
) -> anyhow::Result<(String, i64)> {
    let raw = format!("rf_{}", token::random_token());
    let hash = token::hash(&raw);
    let t = now();
    let tail = &raw[raw.len() - 4..];
    db.write(async |c| store::insert_ref(c, project_id, &hash, tail, single_use, t).await)
        .await?;
    Ok((raw, t))
}

pub struct NewTicket {
    pub title: String,
    pub description: String,
    pub page_url: Option<String>,
}

impl NewTicket {
    /// Every bad field at once, so a form can mark them all.
    pub fn validate(&self) -> Result<(), TicketError> {
        let mut bad = Vec::new();
        if !(3..=120).contains(&self.title.trim().chars().count()) {
            bad.push(("title", "The title needs 3 to 120 characters."));
        }
        if !(1..=10_000).contains(&self.description.trim().chars().count()) {
            bad.push((
                "description",
                "The description needs 1 to 10 000 characters.",
            ));
        }
        if self
            .page_url
            .as_ref()
            .is_some_and(|u| !crate::platform::valid_http_url(u))
        {
            bad.push((
                "page_url",
                "The page URL must start with http:// or https://.",
            ));
        }
        if bad.is_empty() {
            Ok(())
        } else {
            Err(TicketError::Invalid(bad))
        }
    }
}

pub struct Created {
    pub project_id: i64,
    pub number: i64,
    pub created_at: i64,
}

/// US-24. Order of checks: fields, then the ref ID (and, from a project's own form, that it
/// belongs to that project), then the daily caps, so a revoked ID never learns the volume.
/// One transaction: caps, numbering, insert, the single-use mark and the ticket webhook.
pub async fn submit(
    db: &Db,
    base_url: &str,
    only_project: Option<i64>,
    ref_raw: &str,
    t: &NewTicket,
) -> Result<Created, TicketError> {
    if ref_raw.trim().is_empty() {
        let mut bad = vec![("ref_id", "Enter your ref ID.")];
        if let Err(TicketError::Invalid(more)) = t.validate() {
            bad.extend(more);
        }
        return Err(TicketError::Invalid(bad));
    }
    t.validate()?;
    let hash = token::hash(ref_raw.trim());
    let now = now();
    let today = crate::platform::time::day_start(now);
    db.write(async |c| {
        let Some(r) = store::find_ref(c, &hash).await? else {
            return Ok(Err(TicketError::RefIdInvalid));
        };
        if r.revoked_at.is_some()
            || (r.single_use && r.used_at.is_some())
            || only_project.is_some_and(|p| p != r.project_id)
        {
            return Ok(Err(TicketError::RefIdInvalid));
        }
        let per_ref = settings::get(c, "tickets_per_ref_per_day").await?;
        let per_project = settings::get(c, "tickets_per_project_per_day").await?;
        if store::count_since_for_ref(c, r.id, today).await? >= per_ref
            || store::count_since_for_project(c, r.project_id, today).await? >= per_project
        {
            return Ok(Err(TicketError::DailyCap));
        }
        let number =
            store::insert_ticket(c, r.project_id, store::Filer::Ref(r.id), "triage", t, now)
                .await?;
        if r.single_use {
            store::use_ref(c, r.id, now).await?;
        }
        crate::notify::send_ticket(c, r.project_id, number, t.title.trim(), base_url).await?;
        Ok(Ok(Created {
            project_id: r.project_id,
            number,
            created_at: now,
        }))
    })
    .await?
}

/// A team member files a ticket in any column but Done. No caps and no webhook: those guard
/// against reporter traffic, and the team already knows. Returns the ticket's number.
pub async fn file(
    db: &Db,
    project_id: i64,
    user_id: i64,
    stage: &str,
    t: &NewTicket,
) -> Result<i64, TicketError> {
    t.validate()?;
    if !STAGES[..3].contains(&stage) {
        return Err(TicketError::Invalid(vec![(
            "stage",
            "Pick Triage, Backlog or In progress.",
        )]));
    }
    let at = now();
    Ok(db
        .write(async |c| {
            store::insert_ticket(c, project_id, store::Filer::Member(user_id), stage, t, at).await
        })
        .await?)
}

/// Page `page` (from 1) of one column, newest change first. `caps::TICKETS_PER_COLUMN` a page.
pub async fn column(
    db: &Db,
    project_id: i64,
    stage: &str,
    page: i64,
) -> anyhow::Result<Vec<TicketRow>> {
    let offset = (page.max(1) - 1) * crate::platform::caps::TICKETS_PER_COLUMN;
    store::column_page(&db.read().await?, project_id, stage, offset).await
}

/// The public copy of `column`. Triage is never public (SEC-14), so it comes back empty.
pub async fn public_column(
    db: &Db,
    project_id: i64,
    stage: &str,
    page: i64,
) -> anyhow::Result<Vec<PublicTicket>> {
    if stage == "triage" {
        return Ok(Vec::new());
    }
    let offset = (page.max(1) - 1) * crate::platform::caps::TICKETS_PER_COLUMN;
    store::public_column_page(&db.read().await?, project_id, stage, offset).await
}

/// US-29: a reporter's tickets by ref ID, also after revocation or single use. No descriptions.
pub async fn mine(
    db: &Db,
    ref_raw: &str,
    only_project: Option<i64>,
) -> Result<Vec<MyTicket>, TicketError> {
    let conn = db.read().await?;
    let Some(r) = store::find_ref(&conn, &token::hash(ref_raw.trim())).await? else {
        return Err(TicketError::RefIdInvalid);
    };
    if only_project.is_some_and(|p| p != r.project_id) {
        return Err(TicketError::RefIdInvalid);
    }
    Ok(store::by_ref(&conn, r.id).await?)
}

/// The newest tickets of each stage, and every stage's total.
pub async fn board(
    db: &Db,
    project_id: i64,
) -> anyhow::Result<(Vec<TicketRow>, Vec<(String, i64)>)> {
    let conn = db.read().await?;
    Ok((
        store::board(&conn, project_id).await?,
        store::stage_counts(&conn, project_id).await?,
    ))
}

/// US-28: tickets out of Triage, public fields only (SEC-14).
pub async fn public_board(
    db: &Db,
    project_id: i64,
) -> anyhow::Result<(Vec<PublicTicket>, Vec<(String, i64)>)> {
    let conn = db.read().await?;
    Ok((
        store::public_board(&conn, project_id).await?,
        store::stage_counts(&conn, project_id).await?,
    ))
}

pub async fn find(db: &Db, project_id: i64, number: i64) -> anyhow::Result<Option<TicketRow>> {
    store::find(&db.read().await?, project_id, number).await
}

/// US-25. Done needs a close reason; leaving Done clears it. A move to another column calls the
/// move webhook in the same write.
pub async fn move_stage(
    db: &Db,
    base_url: &str,
    project_id: i64,
    number: i64,
    stage: &str,
    reason: Option<&str>,
) -> Result<bool, TicketError> {
    if !STAGES.contains(&stage) {
        return Err(TicketError::Invalid(vec![("stage", "Pick a column.")]));
    }
    let reason = if stage == "done" {
        match reason.filter(|r| CLOSE_REASONS.contains(r)) {
            Some(r) => Some(r),
            None => return Err(TicketError::NeedReason),
        }
    } else {
        None
    };
    let t = now();
    Ok(db
        .write(async |c| {
            let Some(before) = store::find(c, project_id, number).await? else {
                return Ok(false);
            };
            store::set_stage(c, project_id, number, stage, reason, t).await?;
            let moved = crate::notify::Moved {
                number,
                title: &before.title,
                from: &before.stage,
                to: stage,
                close_reason: reason,
            };
            crate::notify::send_ticket_moved(c, project_id, &moved, base_url).await?;
            Ok(true)
        })
        .await?)
}

/// US-26. An empty title clears it; the public board then shows the reporter's title.
pub async fn set_public_title(
    db: &Db,
    project_id: i64,
    number: i64,
    title: &str,
) -> Result<bool, TicketError> {
    let title = title.trim();
    if title.chars().count() > 120 {
        return Err(TicketError::Invalid(vec![(
            "public_title",
            "The public title needs at most 120 characters.",
        )]));
    }
    let t = now();
    let value = (!title.is_empty()).then_some(title);
    Ok(db
        .write(async |c| store::set_public_title(c, project_id, number, value, t).await)
        .await?)
}

/// US-27. The tickets stay; only new submissions stop.
pub async fn revoke_ref(db: &Db, project_id: i64, number: i64) -> anyhow::Result<bool> {
    let t = now();
    db.write(async |c| store::revoke_ref_of(c, project_id, number, t).await)
        .await
}

/// US-30: tickets still in Triage with an id above `after_id`, for the digest. An id
/// watermark, not a time, so a ticket written in the digest's own second is neither lost nor repeated.
pub async fn new_in_triage_after(
    c: &turso::Connection,
    project_id: i64,
    after_id: i64,
) -> anyhow::Result<(Vec<(i64, String)>, i64)> {
    store::new_in_triage_after(c, project_id, after_id).await
}

pub async fn max_id(c: &turso::Connection) -> anyhow::Result<i64> {
    store::max_id(c).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(title: &str, desc: &str, url: Option<&str>) -> NewTicket {
        NewTicket {
            title: title.into(),
            description: desc.into(),
            page_url: url.map(str::to_owned),
        }
    }

    #[test]
    fn validation_limits() {
        assert!(
            t("Checkout broken", "Nothing happens", None)
                .validate()
                .is_ok()
        );
        assert!(t("ab", "x", None).validate().is_err());
        assert!(t("Fine title", " ", None).validate().is_err());
        assert!(
            t("Fine title", "x", Some("javascript:alert(1)"))
                .validate()
                .is_err()
        );
        assert!(
            t("Fine title", "x", Some("https://shop.example/cart"))
                .validate()
                .is_ok()
        );
    }
}
