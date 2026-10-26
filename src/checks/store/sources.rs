//! SQL that reads monitors and heartbeats together, or a column other modules ask about.

use turso::Connection;

use crate::platform::caps;

/// Reads the `'monitor'` or `'heartbeat'` literal in column `i`.
fn kind_at(r: &turso::Row, i: usize) -> turso::Result<crate::incidents::SourceKind> {
    let raw = r.get::<String>(i)?;
    crate::incidents::SourceKind::parse(&raw)
        .ok_or_else(|| turso::Error::ConversionFailure(format!("unknown source kind {raw}")))
}

/// The project's public monitors and heartbeats, monitors first (the last column orders them).
pub async fn public_items(
    c: &Connection,
    project_id: i64,
) -> anyhow::Result<Vec<crate::checks::PublicItem>> {
    crate::platform::db::query_all(
        c,
        "SELECT 'monitor', m.id, m.name, m.created_at, EXISTS (SELECT 1 FROM check_results WHERE monitor_id = m.id), 0
         FROM monitors m WHERE m.project_id = ?1 AND m.is_public = 1 AND m.paused_at IS NULL
         UNION ALL
         SELECT 'heartbeat', h.id, h.name, h.created_at, h.last_ping_at IS NOT NULL, 1
         FROM heartbeats h WHERE h.project_id = ?1 AND h.is_public = 1 AND h.paused_at IS NULL
         ORDER BY 6, 3 LIMIT ?2",
        (project_id, caps::SOURCES_PER_PROJECT),
        |r| {
            Ok(crate::checks::PublicItem {
                kind: kind_at(r, 0)?,
                id: r.get(1)?,
                name: r.get(2)?,
                created_at: r.get(3)?,
                seen: r.get::<i64>(4)? == 1,
            })
        },
    )
    .await
}

/// Every monitor and heartbeat of a project as (kind, id, name), monitors first (the last
/// column orders them).
pub async fn source_names(
    c: &Connection,
    project_id: i64,
) -> anyhow::Result<Vec<(crate::incidents::SourceKind, i64, String)>> {
    crate::platform::db::query_all(
        c,
        "SELECT 'monitor', id, name, 0 FROM monitors WHERE project_id = ?1
         UNION ALL SELECT 'heartbeat', id, name, 1 FROM heartbeats WHERE project_id = ?1 ORDER BY 4, 3 LIMIT ?2",
        (project_id, caps::SOURCES_PER_PROJECT),
        |r| Ok((kind_at(r, 0)?, r.get(1)?, r.get(2)?)),
    )
    .await
}

/// Whether a monitor's or heartbeat's failures alert; a deleted one counts as on.
pub async fn alert_on(
    c: &Connection,
    kind: crate::incidents::SourceKind,
    id: i64,
) -> anyhow::Result<bool> {
    let table = match kind {
        crate::incidents::SourceKind::Monitor => "monitors",
        crate::incidents::SourceKind::Heartbeat => "heartbeats",
    };
    let on = crate::platform::db::query_i64(
        c,
        &format!("SELECT alert FROM {table} WHERE id = ?1"),
        [id],
    )
    .await?;
    Ok(on.unwrap_or(1) == 1)
}

pub async fn secret_users(c: &Connection, secret_id: i64) -> anyhow::Result<i64> {
    Ok(crate::platform::db::query_i64(
        c,
        "SELECT count(*) FROM monitors WHERE auth_secret_id = ?1 AND auth_kind != 'none'",
        [secret_id],
    )
    .await?
    .unwrap_or(0))
}

/// The projects that hold these sources.
pub async fn projects_of(
    c: &Connection,
    refs: &[crate::incidents::ItemRef],
) -> anyhow::Result<Vec<i64>> {
    use crate::incidents::SourceKind;
    let mut out = Vec::new();
    for (kind, table) in [
        (SourceKind::Monitor, "monitors"),
        (SourceKind::Heartbeat, "heartbeats"),
    ] {
        let ids: Vec<String> = refs
            .iter()
            .filter(|r| r.kind == kind)
            .map(|r| r.id.to_string())
            .collect();
        if ids.is_empty() {
            continue;
        }
        out.extend(
            crate::platform::db::query_all(
                c,
                format!(
                    "SELECT DISTINCT project_id FROM {table} WHERE id IN ({})",
                    ids.join(",")
                ),
                (),
                |r| r.get::<i64>(0),
            )
            .await?,
        );
    }
    Ok(out)
}
