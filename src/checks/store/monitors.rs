//! SQL for monitors.

use turso::Connection;

use super::CheckRow;
use crate::checks::MonitorInput;
use crate::platform::caps;

pub struct Monitor {
    pub id: i64,
    pub project_id: i64,
    pub name: String,
    /// `http`, `tcp`, `dns` or `ping`.
    pub kind: String,
    pub url: String,
    /// `dns` only.
    pub expected_address: Option<String>,
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
    pub auth_kind: String,
    pub auth_secret_id: Option<i64>,
    pub auth_user: Option<String>,
    pub auth_header: Option<String>,
    /// HTTPS only: warn this many days before the certificate expires (v1.1 Q25).
    pub cert_warn_days: Option<i64>,
    /// The `notAfter` the last HTTPS check saw.
    pub cert_expires_at: Option<i64>,
    /// The `notAfter` the last certificate warning was about.
    pub cert_warned_for: Option<i64>,
    /// How many places must fail for a check to count as failed (technical-specs 14.7).
    pub quorum: i64,
    /// How many locations also check it; 0 means this server alone.
    pub located: i64,
    pub next_check_at: i64,
    pub last: Option<CheckRow>,
    pub created_at: i64,
}

impl Monitor {
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
}

const COLS: &str = "m.id, m.project_id, m.name, m.url, m.interval_s, m.timeout_ms, m.expected_status, m.keyword, m.skip_tls_verify, m.severity, m.is_public, m.next_check_at, c.checked_at, c.ok, c.status_code, c.latency_ms, c.error, m.created_at, c.id, m.alert, m.auto_incident, m.method, m.headers, m.body, m.fails_to_open, m.auth_kind, m.auth_secret_id, m.auth_user, m.auth_header, m.kind, m.expected_address, m.cert_warn_days, m.cert_expires_at, m.cert_warned_for, m.quorum, (SELECT count(*) FROM monitor_locations WHERE monitor_id = m.id)";
const FROM: &str = "FROM monitors m LEFT JOIN check_results c ON c.id = (SELECT id FROM check_results WHERE monitor_id = m.id AND location_id IS NULL ORDER BY checked_at DESC, id DESC LIMIT 1)";

fn monitor(r: &turso::Row) -> turso::Result<Monitor> {
    let last = match r.get::<Option<i64>>(12)? {
        Some(at) => Some(CheckRow {
            id: r.get(18)?,
            checked_at: at,
            ok: r.get::<i64>(13)? == 1,
            status_code: r.get(14)?,
            latency_ms: r.get(15)?,
            error: r.get(16)?,
            place: None,
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
        auth_kind: r.get(25)?,
        auth_secret_id: r.get(26)?,
        auth_user: r.get(27)?,
        auth_header: r.get(28)?,
        kind: r.get(29)?,
        expected_address: r.get(30)?,
        cert_warn_days: r.get(31)?,
        cert_expires_at: r.get(32)?,
        cert_warned_for: r.get(33)?,
        quorum: r.get(34)?,
        located: r.get(35)?,
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

/// The columns later migrations added, beside the main write. Sign-in fields another kind does
/// not use are cleared.
async fn set_switches(c: &Connection, id: i64, m: &MonitorInput) -> anyhow::Result<()> {
    let kind = m.auth_kind();
    let trimmed = |v: &Option<String>| {
        v.as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_string)
    };
    c.execute(
        "UPDATE monitors SET alert = ?1, auto_incident = ?2, method = ?3, headers = ?4, body = ?5, fails_to_open = ?7,
           auth_kind = ?8, auth_secret_id = ?9, auth_user = ?10, auth_header = ?11, kind = ?12, expected_address = ?13,
           cert_warn_days = ?14, quorum = ?15
         WHERE id = ?6",
        (
            m.alert,
            m.auto_incident,
            m.method(),
            Some(m.headers.trim()).filter(|h| !h.is_empty()),
            m.body.as_deref().filter(|b| !b.is_empty() && matches!(m.method(), "POST" | "PUT")),
            id,
            m.fails_to_open,
            kind,
            m.auth_secret_id.filter(|_| kind != "none"),
            trimmed(&m.auth_user).filter(|_| kind == "basic"),
            trimmed(&m.auth_header).filter(|_| kind == "header"),
            m.kind(),
            m.expected_address.as_deref(),
            m.cert_warn_days,
            m.quorum.max(1),
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

/// The certificate expiry an HTTPS check saw, and the expiry a warning went out for when it did.
pub async fn set_cert(
    c: &Connection,
    id: i64,
    expires_at: i64,
    warned: bool,
) -> anyhow::Result<()> {
    c.execute(
        "UPDATE monitors SET cert_expires_at = ?1,
           cert_warned_for = CASE WHEN ?2 THEN ?1 ELSE cert_warned_for END
         WHERE id = ?3",
        (expires_at, warned, id),
    )
    .await?;
    Ok(())
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

/// Claims up to `limit` of a location's due checks by moving each one interval ahead, and
/// returns their monitors.
pub async fn claim_for_location(
    c: &Connection,
    location_id: i64,
    at: i64,
    limit: i64,
) -> anyhow::Result<Vec<Monitor>> {
    let due = many(
        c,
        format!(
            "SELECT {COLS} {FROM} JOIN monitor_locations ml ON ml.monitor_id = m.id
             WHERE ml.location_id = ?1 AND ml.next_check_at <= ?2 ORDER BY ml.next_check_at LIMIT ?3"
        ),
        (location_id, at, limit),
    )
    .await?;
    for m in &due {
        c.execute(
            // Claims the probe never answered expire with their newest one: a stale count starts
            // over at 1, so it cannot credit a result to a check that was never handed out. The
            // CASE reads the old claimed_at: SQL evaluates every SET expression against the row
            // as it was (claims_a_probe_never_answered_do_not_credit_a_later_result checks it).
            &format!(
                "UPDATE monitor_locations SET next_check_at = ?1, claimed_at = ?2,
                   open_claims = CASE WHEN claimed_at IS NULL OR claimed_at < ?2 - {} THEN 1
                     ELSE min(open_claims + 1, 2) END
                 WHERE monitor_id = ?3 AND location_id = ?4",
                super::locations::STALE_AFTER
            ),
            (at + m.interval_s, at, m.id, location_id),
        )
        .await?;
    }
    Ok(due)
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

pub async fn count_monitors(c: &Connection, project_id: i64) -> anyhow::Result<i64> {
    Ok(crate::platform::db::query_i64(
        c,
        "SELECT count(*) FROM monitors WHERE project_id = ?1",
        [project_id],
    )
    .await?
    .unwrap_or(0))
}
