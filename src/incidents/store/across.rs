//! SQL for the cross-project Incidents page: one page of incidents from every project, or from
//! the projects one tech lead owns, with the list's search, filters and sort.

use crate::platform::sql::Connection;

use super::{COLS, IncidentRow, row};

/// The page's search, filters and sort, as the URL gives them.
#[derive(Clone, Copy, Debug, Default)]
pub struct AcrossFilter<'a> {
    /// Matches the title, `INC-12`, or the project's name, any case.
    pub q: &'a str,
    /// `open` (or empty), `resolved` or `all`.
    pub state: &'a str,
    /// `critical`, `warning` or empty for both.
    pub severity: &'a str,
    /// `newest`, `oldest`, or empty: critical first, then newest.
    pub sort: &'a str,
    /// Unix bounds from `platform::time::day_range`: an incident shows when it was open in them.
    pub range: (Option<i64>, Option<i64>),
}

/// An incident open at some time in `[from, to)`: `from` and `to` are the next two parameters
/// (NULL for no bound). Shared with the project's own list.
pub(super) fn range_sql(from: usize, to: usize) -> String {
    format!(
        "(?{from} IS NULL OR i.resolved_at IS NULL OR i.resolved_at > ?{from}) \
         AND (?{to} IS NULL OR i.started_at < ?{to})"
    )
}

/// `?1` owner (NULL: every project), `?2` search, `?3` severity, `?4` and `?5` the range. The
/// `projects` reads are read-only: the owner scope and the name search.
fn filter_sql(f: &AcrossFilter<'_>) -> String {
    let state = match f.state {
        "resolved" => "i.resolved_at IS NOT NULL",
        "all" => "1",
        _ => "i.resolved_at IS NULL",
    };
    format!(
        "{state} AND (?1 IS NULL OR i.project_id IN (SELECT id FROM projects WHERE owner_id = ?1))
         AND (?2 = '' OR instr(lower(i.title), ?2) > 0 OR ('inc-' || i.id) = ?2
              OR i.project_id IN (SELECT id FROM projects WHERE instr(lower(name), ?2) > 0))
         AND (?3 = '' OR i.severity = ?3) AND {}",
        range_sql(4, 5)
    )
}

fn order_sql(sort: &str) -> &'static str {
    match sort {
        "newest" => "i.started_at DESC, i.id DESC",
        "oldest" => "i.started_at ASC, i.id ASC",
        _ => "(i.severity = 'critical') DESC, i.started_at DESC, i.id DESC",
    }
}

fn search(q: &str) -> String {
    q.trim().to_lowercase().chars().take(100).collect()
}

pub async fn across(
    c: &Connection,
    owner: Option<i64>,
    f: &AcrossFilter<'_>,
    limit: i64,
    offset: i64,
) -> anyhow::Result<Vec<(i64, IncidentRow)>> {
    crate::platform::db::query_all(
        c,
        format!(
            "SELECT {COLS}, i.project_id FROM incidents i WHERE {} ORDER BY {} LIMIT ?6 OFFSET ?7",
            filter_sql(f),
            order_sql(f.sort)
        ),
        (
            owner,
            search(f.q),
            f.severity,
            f.range.0,
            f.range.1,
            limit,
            offset,
        ),
        |r| Ok((r.get(11)?, row(r)?)),
    )
    .await
}

pub async fn count_across(
    c: &Connection,
    owner: Option<i64>,
    f: &AcrossFilter<'_>,
) -> anyhow::Result<i64> {
    Ok(crate::platform::db::query_i64(
        c,
        &format!("SELECT count(*) FROM incidents i WHERE {}", filter_sql(f)),
        (owner, search(f.q), f.severity, f.range.0, f.range.1),
    )
    .await?
    .unwrap_or(0))
}
