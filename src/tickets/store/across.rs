//! SQL for the cross-project Tickets page: tickets waiting in Triage in every project, or in the
//! projects one tech lead owns, with the page's search and sort.

use crate::platform::sql::Connection;

/// A new ticket in Triage, for the cross-project Tickets list.
#[derive(serde::Serialize)]
pub struct TriageRow {
    #[serde(skip)]
    pub project_id: i64,
    pub number: i64,
    pub title: String,
    pub created_at: i64,
}

/// `?1` owner (NULL: every project), `?2` search on the title, `#12`, or the project's name.
/// The `projects` reads are read-only: the owner scope and the name search.
const FILTER: &str = "stage = 'triage'
    AND (?1 IS NULL OR project_id IN (SELECT id FROM projects WHERE owner_id = ?1))
    AND (?2 = '' OR instr(lower(title), ?2) > 0 OR ('#' || number) = ?2
         OR project_id IN (SELECT id FROM projects WHERE instr(lower(name), ?2) > 0))";

fn search(q: &str) -> String {
    q.trim().to_lowercase().chars().take(100).collect()
}

/// `oldest` reads the longest waiting first; anything else, the newest.
pub async fn triage_across(
    c: &Connection,
    owner: Option<i64>,
    q: &str,
    sort: &str,
    limit: i64,
    offset: i64,
) -> anyhow::Result<Vec<TriageRow>> {
    let order = if sort == "oldest" {
        "created_at ASC, id ASC"
    } else {
        "created_at DESC, id DESC"
    };
    crate::platform::db::query_all(
        c,
        format!("SELECT project_id, number, title, created_at FROM tickets WHERE {FILTER} ORDER BY {order} LIMIT ?3 OFFSET ?4"),
        (owner, search(q), limit, offset),
        |r| {
            Ok(TriageRow {
                project_id: r.get(0)?,
                number: r.get(1)?,
                title: r.get(2)?,
                created_at: r.get(3)?,
            })
        },
    )
    .await
}

pub async fn count_triage_across(
    c: &Connection,
    owner: Option<i64>,
    q: &str,
) -> anyhow::Result<i64> {
    Ok(crate::platform::db::query_i64(
        c,
        &format!("SELECT count(*) FROM tickets WHERE {FILTER}"),
        (owner, search(q)),
    )
    .await?
    .unwrap_or(0))
}
