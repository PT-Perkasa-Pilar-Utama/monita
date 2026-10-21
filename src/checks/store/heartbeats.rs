//! SQL for heartbeats and pings.

use turso::Connection;

use crate::checks::heartbeat::{Heartbeat, HeartbeatInput, LastPing, PingKind, PingRow};
use crate::platform::caps;

const COLS: &str = "h.id, h.project_id, h.name, h.token, h.period_s, h.grace_s, h.severity, h.is_public, h.last_ping_at, h.deadline_at, h.created_at, h.alert, h.auto_incident, h.paused_at";

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
        alert: r.get::<i64>(11)? == 1,
        auto_incident: r.get::<i64>(12)? == 1,
        last_ping_at: r.get(8)?,
        deadline_at: r.get(9)?,
        created_at: r.get(10)?,
        paused_at: r.get(13)?,
    })
}

async fn many(
    c: &Connection,
    sql: String,
    params: impl turso::params::IntoParams,
) -> anyhow::Result<Vec<Heartbeat>> {
    crate::platform::db::query_all(c, sql, params, row).await
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
    let id = c.last_insert_rowid();
    set_switches(c, id, input).await?;
    Ok(id)
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
    if changed > 0 {
        set_switches(c, id, input).await?;
    }
    Ok(changed > 0)
}

/// Whether a heartbeat's misses and fail pings alert and open an incident, beside the main write.
async fn set_switches(c: &Connection, id: i64, input: &HeartbeatInput) -> anyhow::Result<()> {
    c.execute(
        "UPDATE heartbeats SET alert = ?1, auto_incident = ?2 WHERE id = ?3",
        (input.alert, input.auto_incident, id),
    )
    .await?;
    Ok(())
}

pub async fn delete(c: &Connection, project_id: i64, id: i64) -> anyhow::Result<()> {
    c.execute(
        "DELETE FROM heartbeats WHERE id = ?1 AND project_id = ?2",
        (id, project_id),
    )
    .await?;
    Ok(())
}

/// The newest ping of each heartbeat in the project, of either kind, by heartbeat id.
pub async fn last_pings(c: &Connection, project_id: i64) -> anyhow::Result<Vec<(i64, LastPing)>> {
    crate::platform::db::query_all(
        c,
        "SELECT h.id, p.kind, p.cause, p.received_at FROM heartbeats h
         JOIN pings p ON p.id = (SELECT id FROM pings WHERE heartbeat_id = h.id ORDER BY received_at DESC, id DESC LIMIT 1)
         WHERE h.project_id = ?1 LIMIT ?2",
        (project_id, caps::HEARTBEATS_PER_PROJECT),
        |r| {
            Ok((
                r.get(0)?,
                LastPing {
                    ok: r.get::<String>(1)? == "ok",
                    cause: r.get(2)?,
                    at: r.get(3)?,
                },
            ))
        },
    )
    .await
}

pub async fn list(c: &Connection, project_id: i64) -> anyhow::Result<Vec<Heartbeat>> {
    many(
        c,
        format!("SELECT {COLS} FROM heartbeats h WHERE h.project_id = ?1 ORDER BY h.name LIMIT ?2"),
        (project_id, caps::HEARTBEATS_PER_PROJECT),
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

/// Up to `limit` pings since `since`, newest first, older than the `(received_at, id)` cursor.
pub async fn recent_pings(
    c: &Connection,
    id: i64,
    since: i64,
    before: Option<(i64, i64)>,
    limit: i64,
) -> anyhow::Result<Vec<PingRow>> {
    let (t, pid) = before.unwrap_or((i64::MAX, i64::MAX));
    crate::platform::db::query_all(c, "SELECT id, received_at, kind, source_ip, cause FROM pings WHERE heartbeat_id = ?1 AND received_at >= ?2
            AND (received_at < ?3 OR (received_at = ?3 AND id < ?4)) ORDER BY received_at DESC, id DESC LIMIT ?5",
            (id, since, t, pid, limit), |r| Ok(PingRow {
            id: r.get(0)?,
            received_at: r.get(1)?,
            kind: r.get(2)?,
            source_ip: r.get(3)?,
            cause: r.get(4)?,
        })).await
}

pub async fn insert_ping(
    c: &Connection,
    id: i64,
    kind: PingKind,
    source_ip: Option<&str>,
    cause: Option<&str>,
    at: i64,
) -> anyhow::Result<()> {
    let kind_str = match kind {
        PingKind::Ok => "ok",
        PingKind::Fail => "fail",
    };
    c.execute(
        "INSERT INTO pings (heartbeat_id, received_at, kind, source_ip, cause) VALUES (?1, ?2, ?3, ?4, ?5)",
        (id, at, kind_str, source_ip, cause),
    )
    .await?;
    if kind == PingKind::Ok {
        c.execute("UPDATE heartbeats SET last_ping_at = ?1, deadline_at = ?1 + period_s + grace_s, held_cause = NULL WHERE id = ?2", (at, id)).await?;
    }
    Ok(())
}

/// A fresh deadline from `at`, as if the job had pinged then, without counting a ping.
/// Pauses, or resumes with a fresh deadline from `at`.
pub async fn set_paused(c: &Connection, id: i64, paused: bool, at: i64) -> anyhow::Result<()> {
    if paused {
        c.execute(
            "UPDATE heartbeats SET paused_at = ?1 WHERE id = ?2",
            (at, id),
        )
        .await?;
        return Ok(());
    }
    c.execute("UPDATE heartbeats SET paused_at = NULL WHERE id = ?1", [id])
        .await?;
    rearm(c, id, at).await
}

pub async fn rearm(c: &Connection, id: i64, at: i64) -> anyhow::Result<()> {
    c.execute(
        "UPDATE heartbeats SET deadline_at = ?1 + period_s + grace_s, held_cause = NULL WHERE id = ?2",
        (at, id),
    )
    .await?;
    Ok(())
}

/// A failure reported in a maintenance window: the sweep looks again at `at` and, once the window
/// is over, opens the incident with `cause`. A heartbeat with no deadline is left alone: its miss
/// was already reported and it is down, so the failure adds nothing.
pub async fn hold_failure(c: &Connection, id: i64, at: i64, cause: &str) -> anyhow::Result<()> {
    c.execute(
        "UPDATE heartbeats SET deadline_at = ?1, held_cause = ?2 WHERE id = ?3 AND deadline_at IS NOT NULL",
        (at, cause, id),
    )
    .await?;
    Ok(())
}

/// The failure a held heartbeat reported, if any, and forgets it.
pub async fn take_held_cause(c: &Connection, id: i64) -> anyhow::Result<Option<String>> {
    let cause = crate::platform::db::query_all(
        c,
        "SELECT held_cause FROM heartbeats WHERE id = ?1 AND held_cause IS NOT NULL",
        [id],
        |r| r.get::<String>(0),
    )
    .await?
    .pop();
    if cause.is_some() {
        c.execute(
            "UPDATE heartbeats SET held_cause = NULL WHERE id = ?1",
            [id],
        )
        .await?;
    }
    Ok(cause)
}

/// Sets the deadline to `at`: the sweep looks at the heartbeat again then.
pub async fn hold(c: &Connection, id: i64, at: i64) -> anyhow::Result<()> {
    c.execute(
        "UPDATE heartbeats SET deadline_at = ?1 WHERE id = ?2",
        (at, id),
    )
    .await?;
    Ok(())
}

pub async fn any_missed(c: &Connection, at: i64) -> anyhow::Result<bool> {
    let n = crate::platform::db::query_i64(
        c,
        "SELECT EXISTS (SELECT 1 FROM heartbeats WHERE deadline_at <= ?1 AND paused_at IS NULL)",
        [at],
    )
    .await?;
    Ok(n == Some(1))
}

/// Returns up to `limit` heartbeats past their deadline, oldest deadline first, and clears
/// those deadlines.
pub async fn take_missed(c: &Connection, at: i64, limit: i64) -> anyhow::Result<Vec<Heartbeat>> {
    let missed = many(
        c,
        format!("SELECT {COLS} FROM heartbeats h WHERE h.deadline_at <= ?1 AND h.paused_at IS NULL ORDER BY h.deadline_at, h.id LIMIT ?2"),
        (at, limit),
    )
    .await?;
    for hb in &missed {
        c.execute(
            "UPDATE heartbeats SET deadline_at = NULL WHERE id = ?1",
            [hb.id],
        )
        .await?;
    }
    Ok(missed)
}

pub async fn count(c: &Connection, project_id: i64) -> anyhow::Result<i64> {
    Ok(crate::platform::db::query_i64(
        c,
        "SELECT count(*) FROM heartbeats WHERE project_id = ?1",
        [project_id],
    )
    .await?
    .unwrap_or(0))
}
