//! Batch and cross-incident reads: items, updates, item states, intervals, the public list.

use super::*;

/// Integer ids joined for `IN (...)`. Rust formats them, so inlining cannot inject SQL.
fn id_list(ids: impl IntoIterator<Item = i64>) -> String {
    ids.into_iter()
        .map(|id| id.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

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
                "SELECT incident_id, source_kind, source_id FROM incident_items WHERE incident_id IN ({})",
                id_list(incident_ids.iter().copied())
            ),
            (),
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
    Ok(updates_of(c, &[incident_id])
        .await?
        .remove(&incident_id)
        .unwrap_or_default())
}

/// Each incident's updates, newest first.
pub async fn updates_of(
    c: &Connection,
    incident_ids: &[i64],
) -> anyhow::Result<HashMap<i64, Vec<UpdateRow>>> {
    let mut rows = c
        .query(
            format!(
                "SELECT s.incident_id, s.label, s.message, u.email, s.created_at FROM status_updates s LEFT JOIN users u ON u.id = s.author_id
                 WHERE s.incident_id IN ({}) ORDER BY s.created_at DESC, s.id DESC",
                id_list(incident_ids.iter().copied())
            ),
            (),
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
             WHERE i.project_id = ?1 AND i.resolved_at IS NULL",
            [project_id],
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

/// (item, started_at, resolved_at) of automatic incidents still open or resolved after `since`.
pub async fn auto_intervals(
    c: &Connection,
    project_id: Option<i64>,
    since: i64,
) -> anyhow::Result<Vec<(ItemRef, i64, Option<i64>)>> {
    let mut rows = c
        .query(
            "SELECT it.source_kind, it.source_id, i.started_at, i.resolved_at FROM incidents i JOIN incident_items it ON it.incident_id = i.id
             WHERE (?1 IS NULL OR i.project_id = ?1) AND i.origin = 'auto' AND (i.resolved_at IS NULL OR i.resolved_at >= ?2)",
            (project_id, since),
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

/// Incidents started since `since` or still open that name one of `items`, newest 200 first.
pub async fn since_touching(
    c: &Connection,
    project_id: i64,
    since: i64,
    items: &[ItemRef],
) -> anyhow::Result<Vec<IncidentRow>> {
    let of = |kind: SourceKind| id_list(items.iter().filter(|i| i.kind == kind).map(|i| i.id));
    let mut rows = c
        .query(
            format!(
                "SELECT {COLS} FROM incidents i WHERE i.project_id = ?1 AND (i.started_at >= ?2 OR i.resolved_at IS NULL)
                 AND EXISTS (SELECT 1 FROM incident_items it WHERE it.incident_id = i.id AND (
                   (it.source_kind = 'monitor' AND it.source_id IN ({})) OR (it.source_kind = 'heartbeat' AND it.source_id IN ({}))))
                 ORDER BY i.started_at DESC LIMIT 200",
                of(SourceKind::Monitor),
                of(SourceKind::Heartbeat)
            ),
            (project_id, since),
        )
        .await?;
    let mut out = Vec::new();
    while let Some(r) = rows.next().await? {
        out.push(row(&r)?);
    }
    Ok(out)
}
