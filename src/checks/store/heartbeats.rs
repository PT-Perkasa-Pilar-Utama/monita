//! SQL for heartbeats and pings.

use turso::Connection;

use crate::checks::heartbeat::{Heartbeat, HeartbeatInput, PingKind, PingRow};

const COLS: &str = "h.id, h.project_id, h.name, h.token, h.period_s, h.grace_s, h.severity, h.is_public, h.last_ping_at, h.deadline_at,
    (SELECT kind FROM pings WHERE heartbeat_id = h.id ORDER BY received_at DESC, id DESC LIMIT 1)";

fn row(r: &turso::Row) -> turso::Result<Heartbeat> {
    Ok(Heartbeat {
        id: r.get(0)?,
        project_id: r.get(1)?,
        name: r.get(2)?,
        token: r.get(3)?,
        period_s: r.get(4)?,
        grace_s: r.get(5)?,
        severity: r.get(6)?,
        is_public: r.get::<i64>(7)? == 1,
        last_ping_at: r.get(8)?,
        deadline_at: r.get(9)?,
        last_kind: r.get(10)?,
    })
}

async fn many(
    c: &Connection,
    sql: String,
    params: impl turso::params::IntoParams,
) -> anyhow::Result<Vec<Heartbeat>> {
    let mut rows = c.query(sql, params).await?;
    let mut out = Vec::new();
    while let Some(r) = rows.next().await? {
        out.push(row(&r)?);
    }
    Ok(out)
}

pub async fn insert(
    c: &Connection,
    project_id: i64,
    input: &HeartbeatInput,
    tok: &str,
    deadline: i64,
) -> anyhow::Result<i64> {
    c.execute(
        "INSERT INTO heartbeats (project_id, name, token, period_s, grace_s, severity, is_public, deadline_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        (project_id, input.name.trim(), tok, input.period_s, input.grace_s, input.severity.as_str(), input.is_public, deadline),
    )
    .await?;
    Ok(c.last_insert_rowid())
}

pub async fn update(
    c: &Connection,
    project_id: i64,
    id: i64,
    input: &HeartbeatInput,
) -> anyhow::Result<bool> {
    let changed = c
        .execute(
            "UPDATE heartbeats SET name = ?1, period_s = ?2, grace_s = ?3, severity = ?4, is_public = ?5,
               deadline_at = CASE WHEN deadline_at IS NULL THEN NULL ELSE coalesce(last_ping_at, created_at) + ?2 + ?3 END,
               updated_at = unixepoch()
             WHERE id = ?6 AND project_id = ?7",
            (input.name.trim(), input.period_s, input.grace_s, input.severity.as_str(), input.is_public, id, project_id),
        )
        .await?;
    Ok(changed > 0)
}

pub async fn delete(c: &Connection, project_id: i64, id: i64) -> anyhow::Result<()> {
    c.execute(
        "DELETE FROM heartbeats WHERE id = ?1 AND project_id = ?2",
        (id, project_id),
    )
    .await?;
    Ok(())
}

pub async fn list(c: &Connection, project_id: i64) -> anyhow::Result<Vec<Heartbeat>> {
    many(
        c,
        format!("SELECT {COLS} FROM heartbeats h WHERE h.project_id = ?1 ORDER BY h.name"),
        [project_id],
    )
    .await
}

pub async fn find(c: &Connection, project_id: i64, id: i64) -> anyhow::Result<Option<Heartbeat>> {
    Ok(many(
        c,
        format!("SELECT {COLS} FROM heartbeats h WHERE h.project_id = ?1 AND h.id = ?2"),
        (project_id, id),
    )
    .await?
    .pop())
}

pub async fn find_by_token(c: &Connection, tok: &str) -> anyhow::Result<Option<Heartbeat>> {
    Ok(many(
        c,
        format!("SELECT {COLS} FROM heartbeats h WHERE h.token = ?1"),
        [tok],
    )
    .await?
    .pop())
}

pub async fn recent_pings(
    c: &Connection,
    id: i64,
    since: i64,
    limit: i64,
) -> anyhow::Result<Vec<PingRow>> {
    let mut rows = c
        .query(
            "SELECT received_at, kind, source_ip FROM pings WHERE heartbeat_id = ?1 AND received_at >= ?2 ORDER BY received_at DESC, id DESC LIMIT ?3",
            (id, since, limit),
        )
        .await?;
    let mut out = Vec::new();
    while let Some(r) = rows.next().await? {
        out.push(PingRow {
            received_at: r.get(0)?,
            kind: r.get(1)?,
            source_ip: r.get(2)?,
        });
    }
    Ok(out)
}

pub async fn insert_ping(
    c: &Connection,
    id: i64,
    kind: PingKind,
    source_ip: Option<&str>,
    at: i64,
) -> anyhow::Result<()> {
    let kind_str = match kind {
        PingKind::Ok => "ok",
        PingKind::Fail => "fail",
    };
    c.execute(
        "INSERT INTO pings (heartbeat_id, received_at, kind, source_ip) VALUES (?1, ?2, ?3, ?4)",
        (id, at, kind_str, source_ip),
    )
    .await?;
    if kind == PingKind::Ok {
        c.execute("UPDATE heartbeats SET last_ping_at = ?1, deadline_at = ?1 + period_s + grace_s WHERE id = ?2", (at, id)).await?;
    }
    Ok(())
}

/// Returns the heartbeats past their deadline and clears those deadlines.
pub async fn take_missed(c: &Connection, at: i64) -> anyhow::Result<Vec<i64>> {
    let mut ids = Vec::new();
    {
        let mut rows = c
            .query("SELECT id FROM heartbeats WHERE deadline_at <= ?1", [at])
            .await?;
        while let Some(r) = rows.next().await? {
            ids.push(r.get::<i64>(0)?);
        }
    }
    for id in &ids {
        c.execute(
            "UPDATE heartbeats SET deadline_at = NULL WHERE id = ?1",
            [*id],
        )
        .await?;
    }
    Ok(ids)
}
