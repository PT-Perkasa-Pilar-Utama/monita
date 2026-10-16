//! SQL for monitors and check_results.

use turso::Connection;

use crate::checks::{CheckResult, MonitorInput};

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
        }
    }

    pub fn source(&self) -> crate::incidents::Source {
        crate::incidents::Source {
            kind: crate::incidents::SourceKind::Monitor,
            id: self.id,
            project_id: self.project_id,
            name: self.name.clone(),
            severity: self.severity.clone(),
        }
    }

    pub fn target(&self) -> crate::checks::probe::Target<'_> {
        crate::checks::probe::Target {
            url: &self.url,
            timeout_ms: self.timeout_ms,
            expected_status: &self.expected_status,
            keyword: self.keyword.as_deref(),
            skip_tls_verify: self.skip_tls_verify,
        }
    }
}

pub struct CheckRow {
    pub checked_at: i64,
    pub ok: bool,
    pub status_code: Option<i64>,
    pub latency_ms: Option<i64>,
    pub error: Option<String>,
}

const COLS: &str = "m.id, m.project_id, m.name, m.url, m.interval_s, m.timeout_ms, m.expected_status, m.keyword, m.skip_tls_verify, m.severity, m.is_public, m.next_check_at, c.checked_at, c.ok, c.status_code, c.latency_ms, c.error, m.created_at";
const FROM: &str = "FROM monitors m LEFT JOIN check_results c ON c.id = (SELECT id FROM check_results WHERE monitor_id = m.id ORDER BY checked_at DESC, id DESC LIMIT 1)";

fn monitor(r: &turso::Row) -> turso::Result<Monitor> {
    let last = match r.get::<Option<i64>>(12)? {
        Some(at) => Some(CheckRow {
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
    Ok(c.last_insert_rowid())
}

/// Returns false when the monitor is not in the project.
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
    let mut rows = c.query(sql, params).await?;
    let mut out = Vec::new();
    while let Some(r) = rows.next().await? {
        out.push(monitor(&r)?);
    }
    Ok(out)
}

pub async fn list_monitors(c: &Connection, project_id: i64) -> anyhow::Result<Vec<Monitor>> {
    many(
        c,
        format!("SELECT {COLS} {FROM} WHERE m.project_id = ?1 ORDER BY m.name"),
        [project_id],
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
    before: i64,
    limit: i64,
) -> anyhow::Result<Vec<CheckRow>> {
    let mut rows = c
        .query(
            "SELECT checked_at, ok, status_code, latency_ms, error FROM check_results
             WHERE monitor_id = ?1 AND checked_at >= ?2 AND checked_at < ?3
             ORDER BY checked_at DESC, id DESC LIMIT ?4",
            (monitor_id, since, before, limit),
        )
        .await?;
    let mut out = Vec::new();
    while let Some(r) = rows.next().await? {
        out.push(CheckRow {
            checked_at: r.get(0)?,
            ok: r.get::<i64>(1)? == 1,
            status_code: r.get(2)?,
            latency_ms: r.get(3)?,
            error: r.get(4)?,
        });
    }
    Ok(out)
}
