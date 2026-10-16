//! Downtime per day and the public incident history (US-21, US-22, US-32).
//! Downtime is the overlap of automatic incidents with each UTC day; manual incidents are not downtime.

use std::collections::HashMap;

use super::{ItemRef, SourceKind, store};
use crate::platform::db::Db;

/// Seconds of downtime in each of `days` UTC days starting at `first_day` (a midnight).
/// `intervals` are (start, end); an open incident passes `now` as its end.
pub fn per_day(intervals: &[(i64, i64)], first_day: i64, days: usize) -> Vec<i64> {
    (0..days)
        .map(|d| {
            let (lo, hi) = (
                first_day + d as i64 * 86_400,
                first_day + (d as i64 + 1) * 86_400,
            );
            intervals
                .iter()
                .map(|&(s, e)| (e.min(hi) - s.max(lo)).max(0))
                .sum::<i64>()
                .min(86_400)
        })
        .collect()
}

/// Automatic-incident intervals per item since `since`, open ones ending at `now`.
pub async fn intervals(
    db: &Db,
    project_id: i64,
    since: i64,
    now: i64,
) -> anyhow::Result<HashMap<(SourceKind, i64), Vec<(i64, i64)>>> {
    let rows = store::auto_intervals(&db.read().await?, project_id, since).await?;
    let mut out: HashMap<(SourceKind, i64), Vec<(i64, i64)>> = HashMap::new();
    for (item, start, end) in rows {
        out.entry((item.kind, item.id))
            .or_default()
            .push((start, end.unwrap_or(now)));
    }
    Ok(out)
}

/// One source's automatic-incident intervals since `since`, open ones ending at `now`.
pub async fn source_intervals(
    db: &Db,
    kind: SourceKind,
    id: i64,
    since: i64,
    now: i64,
) -> anyhow::Result<Vec<(i64, i64)>> {
    let rows = store::source_intervals(&db.read().await?, kind, id, since).await?;
    Ok(rows
        .into_iter()
        .map(|(s, e)| (s, e.unwrap_or(now)))
        .collect())
}

pub struct PublicUpdate {
    pub label: String,
    pub message: String,
    pub at: i64,
}

pub struct PublicIncident {
    pub title: String,
    pub origin: String,
    pub severity: String,
    pub started_at: i64,
    pub resolved_at: Option<i64>,
    pub items: Vec<ItemRef>,
    pub updates: Vec<PublicUpdate>,
}

/// Incidents since `since` that touch a public item, newest first. Items are cut down to the
/// public ones. Monita's own "Detected" entry drops its cause, which can name internal hosts;
/// TL messages show as written. No author is ever included.
pub async fn public(
    db: &Db,
    project_id: i64,
    public: &[ItemRef],
    since: i64,
) -> anyhow::Result<Vec<PublicIncident>> {
    let conn = db.read().await?;
    let mut out = Vec::new();
    for row in store::since(&conn, project_id, since).await? {
        let items: Vec<ItemRef> = store::items(&conn, row.id)
            .await?
            .into_iter()
            .filter(|i| public.contains(i))
            .collect();
        if items.is_empty() {
            continue;
        }
        let updates = store::updates(&conn, row.id)
            .await?
            .into_iter()
            .map(|u| PublicUpdate {
                message: if u.author_email.is_none() && u.label == "investigating" {
                    "Detected.".into()
                } else {
                    u.message
                },
                label: u.label,
                at: u.created_at,
            })
            .collect();
        out.push(PublicIncident {
            title: row.title,
            origin: row.origin,
            severity: row.severity,
            started_at: row.started_at,
            resolved_at: row.resolved_at,
            items,
            updates,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: i64 = 86_400;

    #[test]
    fn splits_an_incident_across_days_and_caps_a_day() {
        // 2 h before midnight to 1 h after, then a full extra day, then an overlap.
        let intervals = [
            (D - 7_200, D + 3_600),
            (2 * D, 4 * D),
            (2 * D + 10, 2 * D + 20),
        ];
        assert_eq!(per_day(&intervals, 0, 5), vec![7_200, 3_600, D, D, 0]);
    }

    #[test]
    fn nothing_before_or_after_the_window_counts() {
        assert_eq!(per_day(&[(0, 10 * D)], 3 * D, 2), vec![D, D]);
        assert_eq!(per_day(&[(0, 100)], D, 1), vec![0]);
    }
}
