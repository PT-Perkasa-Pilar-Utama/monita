//! Batch and cross-incident reads: items, updates, item states, intervals, the public list.

use super::*;

/// Integer ids joined for `IN (...)`. Rust formats them, so inlining cannot inject SQL.
fn id_list(ids: impl IntoIterator<Item = i64>) -> String {
    ids.into_iter()
        .map(|id| id.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

const PER_SOURCE_CAP: i64 = crate::platform::caps::SOURCES_PER_PROJECT;

pub async fn items(c: &Connection, incident_id: i64) -> anyhow::Result<Vec<ItemRef>> {
    Ok(items_of(c, &[incident_id])
        .await?
        .remove(&incident_id)
        .unwrap_or_default())
}

pub async fn items_of(
    c: &Connection,
    incident_ids: &[i64],
) -> anyhow::Result<HashMap<i64, Vec<ItemRef>>> {
    let mut rows = c
        .query(
            format!(
                "SELECT incident_id, source_kind, source_id FROM (SELECT *, row_number() OVER (PARTITION BY incident_id) AS rn
                 FROM incident_items WHERE incident_id IN ({})) WHERE rn <= ?1",
                id_list(incident_ids.iter().copied())
            ),
            [PER_SOURCE_CAP],
        )
        .await?;
    let mut out: HashMap<i64, Vec<ItemRef>> = HashMap::new();
    while let Some(r) = rows.next().await? {
        if let Some(kind) = SourceKind::parse(&r.get::<String>(1)?) {
            out.entry(r.get(0)?).or_default().push(ItemRef {
                kind,
                id: r.get(2)?,
            });
        }
    }
    Ok(out)
}

pub async fn updates(c: &Connection, incident_id: i64) -> anyhow::Result<Vec<UpdateRow>> {
    Ok(updates_of(
        c,
        &[incident_id],
        crate::platform::caps::UPDATES_PER_INCIDENT,
    )
    .await?
    .remove(&incident_id)
    .unwrap_or_default())
}

/// Each incident's newest `per_incident` updates, newest first.
pub async fn updates_of(
    c: &Connection,
    incident_ids: &[i64],
    per_incident: i64,
) -> anyhow::Result<HashMap<i64, Vec<UpdateRow>>> {
    let mut rows = c
        .query(
            format!(
                "SELECT s.incident_id, s.label, s.message, u.email, s.created_at FROM (SELECT *, row_number() OVER (PARTITION BY incident_id ORDER BY created_at DESC, id DESC) AS rn
                 FROM status_updates WHERE incident_id IN ({})) s LEFT JOIN users u ON u.id = s.author_id
                 WHERE s.rn <= ?1 ORDER BY s.created_at DESC, s.id DESC",
                id_list(incident_ids.iter().copied())
            ),
            [per_incident],
        )
        .await?;
    let mut out: HashMap<i64, Vec<UpdateRow>> = HashMap::new();
    while let Some(r) = rows.next().await? {
        out.entry(r.get(0)?).or_default().push(UpdateRow {
            label: r.get(1)?,
            message: r.get(2)?,
            author_email: r.get(3)?,
            created_at: r.get(4)?,
        });
    }
    Ok(out)
}

pub async fn states_for(
    c: &Connection,
    project_id: i64,
) -> anyhow::Result<HashMap<(SourceKind, i64), ItemState>> {
    let mut rows = c
        .query(
            "SELECT it.source_kind, it.source_id, i.origin FROM incidents i JOIN incident_items it ON it.incident_id = i.id
             WHERE i.project_id = ?1 AND i.resolved_at IS NULL LIMIT ?2",
            (project_id, crate::platform::caps::OPEN_ITEMS),
        )
        .await?;
    let mut out = HashMap::new();
    while let Some(r) = rows.next().await? {
        let Some(kind) = SourceKind::parse(&r.get::<String>(0)?) else {
            continue;
        };
        let state = if r.get::<String>(2)? == "auto" {
            ItemState::Down
        } else {
            ItemState::Issue
        };
        let entry = out.entry((kind, r.get::<i64>(1)?)).or_insert(state);
        if state == ItemState::Down {
            *entry = ItemState::Down;
        }
    }
    Ok(out)
}

/// (item, started_at, resolved_at) of a project's automatic incidents still open or resolved
/// after `since`, newest first, at most `caps::SPANS`.
pub async fn auto_intervals(
    c: &Connection,
    project_id: i64,
    since: i64,
) -> anyhow::Result<Vec<(ItemRef, i64, Option<i64>)>> {
    let mut rows = c
        .query(
            "SELECT it.source_kind, it.source_id, i.started_at, i.resolved_at FROM incidents i JOIN incident_items it ON it.incident_id = i.id
             WHERE i.project_id = ?1 AND i.origin = 'auto' AND (i.resolved_at IS NULL OR i.resolved_at >= ?2)
             ORDER BY i.started_at DESC LIMIT ?3",
            (project_id, since, crate::platform::caps::SPANS),
        )
        .await?;
    let mut out = Vec::new();
    while let Some(r) = rows.next().await? {
        if let Some(kind) = SourceKind::parse(&r.get::<String>(0)?) {
            out.push((
                ItemRef {
                    kind,
                    id: r.get(1)?,
                },
                r.get(2)?,
                r.get(3)?,
            ));
        }
    }
    Ok(out)
}

/// (started_at, resolved_at) of one item's automatic incidents that overlap `[since, until)`,
/// newest first, at most `caps::SPANS`.
pub async fn source_spans(
    c: &Connection,
    item: ItemRef,
    since: i64,
    until: i64,
) -> anyhow::Result<Vec<(i64, Option<i64>)>> {
    crate::platform::db::query_all(
        c,
        "SELECT i.started_at, i.resolved_at FROM incidents i JOIN incident_items it ON it.incident_id = i.id
         WHERE it.source_kind = ?1 AND it.source_id = ?2 AND i.origin = 'auto'
           AND (i.resolved_at IS NULL OR i.resolved_at >= ?3) AND i.started_at < ?4
         ORDER BY i.started_at DESC LIMIT ?5",
        (item.kind.as_str(), item.id, since, until, crate::platform::caps::SPANS),
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .await
}

/// Which public incidents to read. Every variant names its own bound in the caller's `limit`.
#[derive(Clone, Copy)]
pub enum Touching {
    /// Started since the moment, or still open: the status JSON's window.
    Window(i64),
    /// Still open, however old.
    Open,
    /// Resolved, and started since the moment.
    ResolvedSince(i64),
    /// Every incident older than the `(started_at, id)` cursor: one page of the history.
    Before(Option<(i64, i64)>),
}

/// Incidents that name one of `items`, newest first, at most `limit`.
pub async fn touching(
    c: &Connection,
    project_id: i64,
    items: &[ItemRef],
    which: Touching,
    limit: i64,
) -> anyhow::Result<Vec<IncidentRow>> {
    let of = |kind: SourceKind| id_list(items.iter().filter(|i| i.kind == kind).map(|i| i.id));
    let (filter, t, id) = match which {
        Touching::Window(since) => ("(i.started_at >= ?2 OR i.resolved_at IS NULL)", since, 0),
        Touching::Open => ("i.resolved_at IS NULL", 0, 0),
        Touching::ResolvedSince(since) => {
            ("i.resolved_at IS NOT NULL AND i.started_at >= ?2", since, 0)
        }
        Touching::Before(cursor) => {
            let (t, id) = cursor.unwrap_or((i64::MAX, i64::MAX));
            (
                "(i.started_at < ?2 OR (i.started_at = ?2 AND i.id < ?3))",
                t,
                id,
            )
        }
    };
    let mut rows = c
        .query(
            format!(
                "SELECT {COLS} FROM incidents i WHERE i.project_id = ?1 AND {filter}
                 AND EXISTS (SELECT 1 FROM incident_items it WHERE it.incident_id = i.id AND (
                   (it.source_kind = 'monitor' AND it.source_id IN ({})) OR (it.source_kind = 'heartbeat' AND it.source_id IN ({}))))
                 ORDER BY i.started_at DESC, i.id DESC LIMIT ?4",
                of(SourceKind::Monitor),
                of(SourceKind::Heartbeat)
            ),
            (project_id, t, id, limit),
        )
        .await?;
    let mut out = Vec::new();
    while let Some(r) = rows.next().await? {
        out.push(row(&r)?);
    }
    Ok(out)
}

/// One item's automatic incidents overlapping `[since, now)`: (count, seconds down inside the
/// window, longest, mean). An open incident runs to `now`. One aggregate row.
pub async fn period_stats(
    c: &Connection,
    item: ItemRef,
    since: i64,
    now: i64,
) -> anyhow::Result<(i64, i64, Option<i64>, Option<i64>)> {
    let row = crate::platform::db::query_all(
        c,
        "SELECT count(*), coalesce(sum(min(coalesce(i.resolved_at, ?4), ?4) - max(i.started_at, ?3)), 0),
                max(coalesce(i.resolved_at, ?4) - i.started_at), CAST(avg(coalesce(i.resolved_at, ?4) - i.started_at) AS INTEGER)
         FROM incidents i JOIN incident_items it ON it.incident_id = i.id
         WHERE it.source_kind = ?1 AND it.source_id = ?2 AND i.origin = 'auto'
           AND coalesce(i.resolved_at, ?4) > ?3 AND i.started_at < ?4",
        (item.kind.as_str(), item.id, since, now),
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    )
    .await?;
    Ok(row.into_iter().next().unwrap_or((0, 0, None, None)))
}

/// The item's newest automatic incident: (started_at, resolved_at).
pub async fn last_auto(
    c: &Connection,
    item: ItemRef,
) -> anyhow::Result<Option<(i64, Option<i64>)>> {
    Ok(crate::platform::db::query_all(
        c,
        "SELECT i.started_at, i.resolved_at FROM incidents i JOIN incident_items it ON it.incident_id = i.id
         WHERE it.source_kind = ?1 AND it.source_id = ?2 AND i.origin = 'auto' ORDER BY i.started_at DESC LIMIT 1",
        (item.kind.as_str(), item.id),
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .await?
    .pop())
}

/// Open incidents of a project, for its tab count.
pub async fn open_count(c: &Connection, project_id: i64) -> anyhow::Result<i64> {
    Ok(crate::platform::db::query_i64(
        c,
        "SELECT count(*) FROM incidents WHERE project_id = ?1 AND resolved_at IS NULL",
        [project_id],
    )
    .await?
    .unwrap_or(0))
}

/// Of `refs`, the sources down with no incident: their auto-incident is off.
pub async fn down_without_incident(
    c: &Connection,
    refs: &[ItemRef],
) -> anyhow::Result<Vec<ItemRef>> {
    let mut out = Vec::new();
    for kind in [SourceKind::Monitor, SourceKind::Heartbeat] {
        let ids: Vec<i64> = refs
            .iter()
            .filter(|r| r.kind == kind)
            .map(|r| r.id)
            .collect();
        if ids.is_empty() {
            continue;
        }
        let mut rows = c
            .query(
                format!(
                    "SELECT source_id FROM incident_sources WHERE source_kind = ?1 AND down_since IS NOT NULL
                     AND open_incident_id IS NULL AND source_id IN ({})",
                    id_list(ids.into_iter())
                ),
                [kind.as_str()],
            )
            .await?;
        while let Some(r) = rows.next().await? {
            out.push(ItemRef {
                kind,
                id: r.get(0)?,
            });
        }
    }
    Ok(out)
}
