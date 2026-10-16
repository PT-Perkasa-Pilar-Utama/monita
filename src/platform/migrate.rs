//! Migration runner. Spec: docs/technical-specs/06-data-model.md section 6.6.
//! Files are embedded at compile time, paths relative to this file. Append only.

use super::db::{Db, query_i64};

const MIGRATIONS: &[(i64, &str)] = &[
    (1, include_str!("../../migrations/0001_init.sql")),
    (2, include_str!("../../migrations/0002_ping_cause.sql")),
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
        db.write(async |c| {
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
}
