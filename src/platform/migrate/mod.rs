//! Migration runner. Spec: docs/technical-specs/06-data-model.md section 6.6.
//! Files are embedded at compile time, paths relative to this file. Append only.

use super::db::{Db, query_i64};

const MIGRATIONS: &[(i64, &str)] = &[
    (1, include_str!("../../../migrations/0001_init.sql")),
    (2, include_str!("../../../migrations/0002_ping_cause.sql")),
    (
        3,
        include_str!("../../../migrations/0003_stable_source_ids.sql"),
    ),
    (
        4,
        include_str!("../../../migrations/0004_retention_indexes.sql"),
    ),
    (
        5,
        include_str!("../../../migrations/0005_project_profile.sql"),
    ),
    (6, include_str!("../../../migrations/0006_team_tickets.sql")),
    (
        7,
        include_str!("../../../migrations/0007_ticket_alerts.sql"),
    ),
    (
        8,
        include_str!("../../../migrations/0008_alert_switches.sql"),
    ),
    (
        9,
        include_str!("../../../migrations/0009_alert_and_auto_incident.sql"),
    ),
    (
        10,
        include_str!("../../../migrations/0010_public_pages.sql"),
    ),
    (
        11,
        include_str!("../../../migrations/0011_ticket_move_webhook.sql"),
    ),
    (
        12,
        include_str!("../../../migrations/0012_rearm_resolved_heartbeats.sql"),
    ),
    (
        13,
        include_str!("../../../migrations/0013_monitor_request.sql"),
    ),
    (
        14,
        include_str!("../../../migrations/0014_monitor_fails_to_open.sql"),
    ),
    (
        15,
        include_str!("../../../migrations/0015_project_secrets.sql"),
    ),
    (
        16,
        include_str!("../../../migrations/0016_show_incidents.sql"),
    ),
    (17, include_str!("../../../migrations/0017_ref_id_tail.sql")),
    (
        18,
        include_str!("../../../migrations/0018_fails_to_open_range.sql"),
    ),
    (
        19,
        include_str!("../../../migrations/0019_merge_report_and_mine.sql"),
    ),
    (
        20,
        include_str!("../../../migrations/0020_project_homepage.sql"),
    ),
    (21, include_str!("../../../migrations/0021_changelog.sql")),
    (
        22,
        include_str!("../../../migrations/0022_supervisor_role.sql"),
    ),
    (
        23,
        include_str!("../../../migrations/0023_owner_alert_recipients.sql"),
    ),
    (24, include_str!("../../../migrations/0024_check_kinds.sql")),
    (
        25,
        include_str!("../../../migrations/0025_webhook_formats.sql"),
    ),
    (
        26,
        include_str!("../../../migrations/0026_maintenance_windows.sql"),
    ),
    (27, include_str!("../../../migrations/0027_two_factor.sql")),
    (
        28,
        include_str!("../../../migrations/0028_certificate_expiry.sql"),
    ),
    (29, include_str!("../../../migrations/0029_ping_checks.sql")),
    (30, include_str!("../../../migrations/0030_locations.sql")),
    (
        31,
        include_str!("../../../migrations/0031_location_claims.sql"),
    ),
    (
        32,
        include_str!("../../../migrations/0032_held_failure.sql"),
    ),
    (
        33,
        include_str!("../../../migrations/0033_webhook_off_without_url.sql"),
    ),
    (
        34,
        include_str!("../../../migrations/0034_user_avatars.sql"),
    ),
    (
        35,
        include_str!("../../../migrations/0035_totp_pending_expiry.sql"),
    ),
    (
        36,
        include_str!("../../../migrations/0036_telegram_bot.sql"),
    ),
    (37, include_str!("../../../migrations/0037_alert_cards.sql")),
    (
        38,
        include_str!("../../../migrations/0038_ticket_alert_formats.sql"),
    ),
    (
        39,
        include_str!("../../../migrations/0039_ticket_cards.sql"),
    ),
    (40, include_str!("../../../migrations/0040_pause.sql")),
    (41, include_str!("../../../migrations/0041_key_scope.sql")),
    (
        42,
        include_str!("../../../migrations/0042_update_author_key.sql"),
    ),
    (43, include_str!("../../../migrations/0043_no_ip_limit.sql")),
    (
        44,
        include_str!("../../../migrations/0044_incident_class.sql"),
    ),
    (45, include_str!("../../../migrations/0045_in_window.sql")),
    (46, include_str!("../../../migrations/0046_slots.sql")),
    (
        47,
        include_str!("../../../migrations/0047_drop_turso_tables.sql"),
    ),
    (48, include_str!("../../../migrations/0048_pins.sql")),
];

pub async fn run(db: &Db) -> anyhow::Result<()> {
    db.write(async |c| {
        c.execute(
            "CREATE TABLE IF NOT EXISTS schema_migrations (version INTEGER PRIMARY KEY, applied_at INTEGER NOT NULL)",
            (),
        )
        .await?;
        Ok(())
    })
    .await?;

    let mut applied_any = false;
    for &(version, sql) in MIGRATIONS {
        let conn = db.read().await?;
        let applied = query_i64(
            &conn,
            "SELECT count(*) FROM schema_migrations WHERE version = ?1",
            [version],
        )
        .await?
        .unwrap_or(0);
        drop(conn);
        if applied > 0 {
            continue;
        }
        // Foreign keys off: a table rebuild drops a parent table, which would cascade otherwise.
        db.write_without_fk(async |c| {
            c.execute_batch(sql).await?;
            c.execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, unixepoch())",
                [version],
            )
            .await?;
            Ok(())
        })
        .await?;
        tracing::info!(version, "migration applied");
        applied_any = true;
    }
    // A migration runs with foreign keys off, so a broken reference would pass unseen. Checked
    // only after a migration ran: a full check reads every row, which slows a big database.
    if applied_any {
        check_references(db).await?;
    }
    Ok(())
}

/// Refuses to go on when a row points at a row that does not exist, and names the first ones.
pub(crate) async fn check_references(db: &Db) -> anyhow::Result<()> {
    let broken = crate::platform::db::query_all(
        &db.read().await?,
        "SELECT \"table\", rowid, parent FROM pragma_foreign_key_check LIMIT 20",
        (),
        |r| {
            Ok(format!(
                "{} row {} points at a missing {}",
                r.get::<String>(0)?,
                r.get::<Option<i64>>(1)?
                    .map_or("?".into(), |id| id.to_string()),
                r.get::<String>(2)?
            ))
        },
    )
    .await?;
    if broken.is_empty() {
        return Ok(());
    }
    for b in &broken {
        tracing::error!(reference = %b, "broken reference after a migration");
    }
    anyhow::bail!(
        "the database has broken references after a migration, first: {}; restore the backup taken before the upgrade",
        broken[0]
    )
}

#[cfg(test)]
mod tests;
