//! SQL for monitors and check_results.

use turso::Connection;

use crate::checks::{CheckResult, MonitorInput};
use crate::platform::caps;

pub struct Monitor {
    pub id: i64,
    pub project_id: i64,
    pub name: String,
    pub url: String,
    pub interval_s: i64,
    pub timeout_ms: i64,
    pub expected_status: String,
    pub keyword: Option<String>,
    pub skip_tls_verify: bool,
    pub severity: String,
    pub is_public: bool,
    pub alert: bool,
    pub auto_incident: bool,
    pub fails_to_open: i64,
    pub method: String,
    pub headers: Option<String>,
    pub body: Option<String>,
    pub next_check_at: i64,
    pub last: Option<CheckRow>,
    pub created_at: i64,
}

impl Monitor {
    pub fn input(&self) -> MonitorInput {
        MonitorInput {
            name: self.name.clone(),
            url: self.url.clone(),
            interval_s: self.interval_s,
            timeout_ms: self.timeout_ms,
            expected_status: self.expected_status.clone(),
            keyword: self.keyword.clone(),
            skip_tls_verify: self.skip_tls_verify,
            severity: self.severity.clone(),
            is_public: self.is_public,
            alert: self.alert,
            auto_incident: self.auto_incident,
            fails_to_open: self.fails_to_open,
            method: self.method.clone(),
            headers: self.headers.clone().unwrap_or_default(),
            body: self.body.clone(),
        }
    }

    pub fn source(&self) -> crate::incidents::Source {
        crate::incidents::Source {
            kind: crate::incidents::SourceKind::Monitor,
            id: self.id,
            project_id: self.project_id,
            name: self.name.clone(),
            severity: self.severity.clone(),
            alert: self.alert,
            auto_incident: self.auto_incident,
            fails_to_open: self.fails_to_open as u32,
        }
    }

    pub fn target(&self) -> crate::checks::probe::Target<'_> {
        crate::checks::probe::Target {
            url: &self.url,
            method: &self.method,
            // Validated on save, so a parse error here means an edited row; send no headers.
            headers: crate::checks::probe::parse_headers(
                self.headers.as_deref().unwrap_or_default(),
            )
            .unwrap_or_default(),
            body: self.body.as_deref(),
            timeout_ms: self.timeout_ms,
            expected_status: &self.expected_status,
            keyword: self.keyword.as_deref(),
            skip_tls_verify: self.skip_tls_verify,
        }
    }
}

#[derive(serde::Serialize)]
pub struct CheckRow {
    pub id: i64,
    pub checked_at: i64,
    pub ok: bool,
    pub status_code: Option<i64>,
    pub latency_ms: Option<i64>,
    pub error: Option<String>,
}

const COLS: &str = "m.id, m.project_id, m.name, m.url, m.interval_s, m.timeout_ms, m.expected_status, m.keyword, m.skip_tls_verify, m.severity, m.is_public, m.next_check_at, c.checked_at, c.ok, c.status_code, c.latency_ms, c.error, m.created_at, c.id, m.alert, m.auto_incident, m.method, m.headers, m.body, m.fails_to_open";
const FROM: &str = "FROM monitors m LEFT JOIN check_results c ON c.id = (SELECT id FROM check_results WHERE monitor_id = m.id ORDER BY checked_at DESC, id DESC LIMIT 1)";

fn monitor(r: &turso::Row) -> turso::Result<Monitor> {
    let last = match r.get::<Option<i64>>(12)? {
        Some(at) => Some(CheckRow {
            id: r.get(18)?,
            checked_at: at,
            ok: r.get::<i64>(13)? == 1,
            status_code: r.get(14)?,
            latency_ms: r.get(15)?,
            error: r.get(16)?,
        }),
        None => None,
    };
    Ok(Monitor {
        id: r.get(0)?,
        project_id: r.get(1)?,
        name: r.get(2)?,
        url: r.get(3)?,
        interval_s: r.get(4)?,
        timeout_ms: r.get(5)?,
        expected_status: r.get(6)?,
        keyword: r.get(7)?,
        skip_tls_verify: r.get::<i64>(8)? == 1,
        severity: r.get(9)?,
        is_public: r.get::<i64>(10)? == 1,
        alert: r.get::<i64>(19)? == 1,
        auto_incident: r.get::<i64>(20)? == 1,
        method: r.get(21)?,
        headers: r.get(22)?,
        body: r.get(23)?,
        fails_to_open: r.get(24)?,
        next_check_at: r.get(11)?,
        last,
        created_at: r.get(17)?,
    })
}

pub async fn insert_monitor(
    c: &Connection,
    project_id: i64,
    m: &MonitorInput,
    next: i64,
) -> anyhow::Result<i64> {
    c.execute(
        "INSERT INTO monitors (project_id, name, url, interval_s, timeout_ms, expected_status, keyword, skip_tls_verify, severity, is_public, next_check_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        (
            project_id,
            m.name.trim(),
            m.url.trim(),
            m.interval_s,
            m.timeout_ms,
            m.expected_status.replace(' ', ""),
            m.keyword.clone(),
            m.skip_tls_verify,
            m.severity.as_str(),
            m.is_public,
            next,
        ),
    )
    .await?;
    let id = c.last_insert_rowid();
    set_switches(c, id, m).await?;
    Ok(id)
}

/// Whether a monitor's failures alert and open an incident, beside the main write.
async fn set_switches(c: &Connection, id: i64, m: &MonitorInput) -> anyhow::Result<()> {
    c.execute(
        "UPDATE monitors SET alert = ?1, auto_incident = ?2, method = ?3, headers = ?4, body = ?5, fails_to_open = ?7 WHERE id = ?6",
        (
            m.alert,
            m.auto_incident,
            m.method(),
            Some(m.headers.trim()).filter(|h| !h.is_empty()),
            m.body.as_deref().filter(|b| !b.is_empty() && matches!(m.method(), "POST" | "PUT")),
            id,
            m.fails_to_open,
        ),
    )
    .await?;
    Ok(())
}

pub async fn update_monitor(
    c: &Connection,
    project_id: i64,
    id: i64,
    m: &MonitorInput,
    next: i64,
) -> anyhow::Result<bool> {
    let changed = c
        .execute(
            "UPDATE monitors SET name = ?1, url = ?2,
               next_check_at = CASE WHEN interval_s = ?3 THEN next_check_at ELSE ?4 END,
               interval_s = ?3, timeout_ms = ?5, expected_status = ?6, keyword = ?7, skip_tls_verify = ?8,
               severity = ?9, is_public = ?10, updated_at = unixepoch()
             WHERE id = ?11 AND project_id = ?12",
            (
                m.name.trim(),
                m.url.trim(),
                m.interval_s,
                next,
                m.timeout_ms,
                m.expected_status.replace(' ', ""),
                m.keyword.clone(),
                m.skip_tls_verify,
                m.severity.as_str(),
                m.is_public,
                id,
                project_id,
            ),
        )
        .await?;
    if changed > 0 {
        set_switches(c, id, m).await?;
    }
    Ok(changed > 0)
}

pub async fn delete_monitor(c: &Connection, project_id: i64, id: i64) -> anyhow::Result<()> {
    c.execute(
        "DELETE FROM monitors WHERE id = ?1 AND project_id = ?2",
        (id, project_id),
    )
    .await?;
    Ok(())
}

async fn many(
    c: &Connection,
    sql: String,
    params: impl turso::params::IntoParams,
) -> anyhow::Result<Vec<Monitor>> {
    crate::platform::db::query_all(c, sql, params, monitor).await
}

pub async fn list_monitors(c: &Connection, project_id: i64) -> anyhow::Result<Vec<Monitor>> {
    many(
        c,
        format!("SELECT {COLS} {FROM} WHERE m.project_id = ?1 ORDER BY m.name LIMIT ?2"),
        (project_id, caps::MONITORS_PER_PROJECT),
    )
    .await
}

pub async fn find_monitor(
    c: &Connection,
    project_id: i64,
    id: i64,
) -> anyhow::Result<Option<Monitor>> {
    Ok(many(
        c,
        format!("SELECT {COLS} {FROM} WHERE m.project_id = ?1 AND m.id = ?2"),
        (project_id, id),
    )
    .await?
    .pop())
}

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
         FROM monitors m WHERE m.project_id = ?1 AND m.is_public = 1
         UNION ALL
         SELECT 'heartbeat', h.id, h.name, h.created_at, h.last_ping_at IS NOT NULL, 1
         FROM heartbeats h WHERE h.project_id = ?1 AND h.is_public = 1
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

/// A read-only probe, so an idle tick never waits for the writer.
pub async fn any_due(c: &Connection, at: i64) -> anyhow::Result<bool> {
    let n = crate::platform::db::query_i64(
        c,
        "SELECT EXISTS (SELECT 1 FROM monitors WHERE next_check_at <= ?1)",
        [at],
    )
    .await?;
    Ok(n == Some(1))
}

/// Claims up to `limit` due monitors by moving their `next_check_at` one interval ahead.
pub async fn claim_due(c: &Connection, at: i64, limit: i64) -> anyhow::Result<Vec<Monitor>> {
    let due = many(
        c,
        format!(
            "SELECT {COLS} {FROM} WHERE m.next_check_at <= ?1 ORDER BY m.next_check_at LIMIT ?2"
        ),
        (at, limit),
    )
    .await?;
    for m in &due {
        c.execute(
            "UPDATE monitors SET next_check_at = ?1 WHERE id = ?2",
            (at + m.interval_s, m.id),
        )
        .await?;
    }
    Ok(due)
}

pub async fn insert_check(c: &Connection, monitor_id: i64, r: &CheckResult) -> anyhow::Result<()> {
    c.execute(
        "INSERT INTO check_results (monitor_id, checked_at, ok, status_code, latency_ms, error) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        (monitor_id, r.checked_at, r.ok, r.status_code.map(i64::from), r.latency_ms, r.error.clone()),
    )
    .await?;
    Ok(())
}

pub async fn recent_checks(
    c: &Connection,
    monitor_id: i64,
    since: i64,
    before: (i64, i64),
    limit: i64,
) -> anyhow::Result<Vec<CheckRow>> {
    crate::platform::db::query_all(
        c,
        "SELECT id, checked_at, ok, status_code, latency_ms, error FROM check_results
         WHERE monitor_id = ?1 AND checked_at >= ?2 AND (checked_at < ?3 OR (checked_at = ?3 AND id < ?4))
         ORDER BY checked_at DESC, id DESC LIMIT ?5",
        (monitor_id, since, before.0, before.1, limit),
        |r| {
            Ok(CheckRow {
                id: r.get(0)?,
                checked_at: r.get(1)?,
                ok: r.get::<i64>(2)? == 1,
                status_code: r.get(3)?,
                latency_ms: r.get(4)?,
                error: r.get(5)?,
            })
        },
    )
    .await
}

pub async fn count_monitors(c: &Connection, project_id: i64) -> anyhow::Result<i64> {
    Ok(crate::platform::db::query_i64(
        c,
        "SELECT count(*) FROM monitors WHERE project_id = ?1",
        [project_id],
    )
    .await?
    .unwrap_or(0))
}
