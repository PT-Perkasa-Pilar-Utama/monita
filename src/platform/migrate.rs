//! Migration runner. Spec: docs/technical-specs/06-data-model.md section 6.6.
//! Files are embedded at compile time, paths relative to this file. Append only.

use super::db::{Db, query_i64};

const MIGRATIONS: &[(i64, &str)] = &[
    (1, include_str!("../../migrations/0001_init.sql")),
    (2, include_str!("../../migrations/0002_ping_cause.sql")),
    (
        3,
        include_str!("../../migrations/0003_stable_source_ids.sql"),
    ),
    (
        4,
        include_str!("../../migrations/0004_retention_indexes.sql"),
    ),
    (5, include_str!("../../migrations/0005_project_profile.sql")),
    (6, include_str!("../../migrations/0006_team_tickets.sql")),
    (7, include_str!("../../migrations/0007_ticket_alerts.sql")),
    (8, include_str!("../../migrations/0008_alert_switches.sql")),
    (
        9,
        include_str!("../../migrations/0009_alert_and_auto_incident.sql"),
    ),
    (10, include_str!("../../migrations/0010_public_pages.sql")),
    (
        11,
        include_str!("../../migrations/0011_ticket_move_webhook.sql"),
    ),
    (
        12,
        include_str!("../../migrations/0012_rearm_resolved_heartbeats.sql"),
    ),
    (
        13,
        include_str!("../../migrations/0013_monitor_request.sql"),
    ),
    (
        14,
        include_str!("../../migrations/0014_monitor_fails_to_open.sql"),
    ),
    (
        15,
        include_str!("../../migrations/0015_project_secrets.sql"),
    ),
    (16, include_str!("../../migrations/0016_show_incidents.sql")),
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
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn migrations_apply_once_and_rerun_cleanly() {
        let dir = tempfile::tempdir().unwrap();
        let db = Db::open(&dir.path().join("t.db")).await.unwrap();
        run(&db).await.unwrap();
        run(&db).await.unwrap();
        let conn = db.read().await.unwrap();
        let n = query_i64(&conn, "SELECT count(*) FROM schema_migrations", ())
            .await
            .unwrap();
        assert_eq!(n, Some(MIGRATIONS.len() as i64));
        let settings = query_i64(&conn, "SELECT count(*) FROM settings", ())
            .await
            .unwrap();
        assert_eq!(settings, Some(6));
    }

    #[tokio::test]
    async fn the_id_rebuild_keeps_rows_and_cascades_and_never_reuses_an_id() {
        let dir = tempfile::tempdir().unwrap();
        let db = Db::open(&dir.path().join("t.db")).await.unwrap();
        db.write(async |c| {
            c.execute(
                "CREATE TABLE schema_migrations (version INTEGER PRIMARY KEY, applied_at INTEGER NOT NULL)",
                (),
            )
            .await?;
            for &(version, sql) in &MIGRATIONS[..2] {
                c.execute_batch(sql).await?;
                c.execute(
                    "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, 0)",
                    [version],
                )
                .await?;
            }
            c.execute_batch(
                "INSERT INTO users (id, email, role) VALUES (1, 'a@x.co', 'tl');
                 INSERT INTO projects (id, owner_id, name, slug) VALUES (1, 1, 'P', 'p');
                 INSERT INTO monitors (id, project_id, name, url, next_check_at) VALUES (1, 1, 'A', 'https://a', 0), (2, 1, 'B', 'https://b', 0);
                 INSERT INTO check_results (monitor_id, checked_at, ok) VALUES (2, 1, 1);
                 INSERT INTO daily_summaries (source_kind, source_id, day, checks_total, checks_ok, downtime_s) VALUES ('monitor', 2, '2026-10-16', 1, 1, 0);
                 DELETE FROM monitors WHERE id = 2;
                 INSERT INTO check_results (monitor_id, checked_at, ok) VALUES (1, 1, 1);",
            )
            .await?;
            Ok(())
        })
        .await
        .unwrap();
        run(&db).await.unwrap();
        let count = async |sql: &str| query_i64(&db.read().await.unwrap(), sql, ()).await.unwrap();
        assert_eq!(
            count("SELECT count(*) FROM check_results").await,
            Some(1),
            "rows survive"
        );
        db.write(async |c| {
            c.execute(
                "INSERT INTO monitors (project_id, name, url, next_check_at) VALUES (1, 'C', 'https://c', 0)",
                (),
            )
            .await?;
            c.execute("DELETE FROM monitors WHERE id = 1", ()).await?;
            Ok(())
        })
        .await
        .unwrap();
        assert_eq!(
            count("SELECT max(id) FROM monitors").await,
            Some(3),
            "id 2 is not reused"
        );
        assert_eq!(
            count("SELECT count(*) FROM check_results").await,
            Some(0),
            "delete still cascades"
        );
    }

    #[tokio::test]
    async fn ticket_stage_and_close_reason_must_agree() {
        let dir = tempfile::tempdir().unwrap();
        let db = Db::open(&dir.path().join("t.db")).await.unwrap();
        run(&db).await.unwrap();
        let setup = db
            .write(async |c| {
                c.execute_batch(
                    "INSERT INTO users (id, email, role) VALUES (1, 'a@x', 'tl');
                     INSERT INTO projects (id, owner_id, name, slug) VALUES (1, 1, 'P', 'p');
                     INSERT INTO ref_ids (id, project_id, token_hash) VALUES (1, 1, 'h');",
                )
                .await?;
                Ok(())
            })
            .await;
        assert!(setup.is_ok(), "{setup:?}");
        let bad = db
            .write(async |c| {
                c.execute(
                    "INSERT INTO tickets (project_id, number, ref_id, title, description, stage) VALUES (1, 1, 1, 't', 'd', 'done')",
                    (),
                )
                .await?;
                Ok(())
            })
            .await;
        assert!(bad.is_err(), "done without close_reason must fail");
    }

    #[tokio::test]
    async fn ticket_alerts_keep_the_warning_webhook_a_project_had_on() {
        let dir = tempfile::tempdir().unwrap();
        let db = Db::open(&dir.path().join("t.db")).await.unwrap();
        db.write_without_fk(async |c| {
            c.execute(
                "CREATE TABLE schema_migrations (version INTEGER PRIMARY KEY, applied_at INTEGER NOT NULL)",
                (),
            )
            .await?;
            for &(version, sql) in &MIGRATIONS[..6] {
                c.execute_batch(sql).await?;
                c.execute(
                    "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, 0)",
                    [version],
                )
                .await?;
            }
            c.execute_batch(
                "INSERT INTO users (id, email, role) VALUES (1, 'a@x.co', 'tl');
                 INSERT INTO projects (id, owner_id, name, slug) VALUES (1, 1, 'On', 'on'), (2, 1, 'Off', 'off'), (3, 1, 'Bare', 'bare');
                 INSERT INTO alert_routes (project_id, severity, webhook_url, use_webhook) VALUES
                   (1, 'warning', 'https://hook/on', 1), (2, 'warning', 'https://hook/off', 0);",
            )
            .await?;
            Ok(())
        })
        .await
        .unwrap();
        run(&db).await.unwrap();
        let rows = crate::platform::db::query_all(
            &db.read().await.unwrap(),
            "SELECT project_id, digest, coalesce(webhook_url, ''), use_webhook FROM ticket_alerts ORDER BY project_id",
            (),
            |r| Ok((r.get::<i64>(0)?, r.get::<i64>(1)?, r.get::<String>(2)?, r.get::<i64>(3)?)),
        )
        .await
        .unwrap();
        assert_eq!(
            rows,
            vec![
                (1, 1, "https://hook/on".to_string(), 1),
                (2, 1, "https://hook/off".to_string(), 0),
                (3, 1, String::new(), 0),
            ]
        );
    }
}
