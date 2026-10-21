//! Migration tests: each runs the real migrations on a temp-file database.

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
    let four = db
        .write(async |c| {
            c.execute(
                "INSERT INTO monitors (project_id, name, url, next_check_at, fails_to_open) VALUES (1, 'D', 'https://d', 0, 4)",
                (),
            )
            .await?;
            Ok(())
        })
        .await;
    assert!(four.is_ok(), "any count from 1 to 99: {four:?}");
    let hundred = db
        .write(async |c| {
            c.execute(
                "INSERT INTO monitors (project_id, name, url, next_check_at, fails_to_open) VALUES (1, 'E', 'https://e', 0, 100)",
                (),
            )
            .await?;
            Ok(())
        })
        .await;
    assert!(hundred.is_err(), "100 is past the range");
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

/// A database with the first `n` migrations, as an older Monita left it.
async fn at(n: usize) -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let db = Db::open(&dir.path().join("t.db")).await.unwrap();
    exec(
        &db,
        "CREATE TABLE schema_migrations (version INTEGER PRIMARY KEY, applied_at INTEGER NOT NULL)",
    )
    .await;
    apply(&db, 0..n).await;
    (dir, db)
}

/// Applies the migrations in `range` as `run` would.
async fn apply(db: &Db, range: std::ops::Range<usize>) {
    db.write_without_fk(async |c| {
        for &(version, sql) in &MIGRATIONS[range.clone()] {
            c.execute_batch(sql).await?;
            c.execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, 0)",
                [version],
            )
            .await?;
        }
        Ok(())
    })
    .await
    .unwrap();
}

async fn exec(db: &Db, sql: &str) {
    db.write_without_fk(async |c| {
        c.execute_batch(sql).await?;
        Ok(())
    })
    .await
    .unwrap();
}

async fn strings(db: &Db, sql: &str) -> Vec<String> {
    crate::platform::db::query_all(&db.read().await.unwrap(), sql, (), |r| r.get::<String>(0))
        .await
        .unwrap()
}

/// Migration 23: a route that sent nothing gets the owner's email; a webhook-only route and a
/// route with recipients keep theirs.
#[tokio::test]
async fn migration_23_gives_silent_routes_the_owners_email() {
    let (_dir, db) = at(22).await;
    exec(
        &db,
        "INSERT INTO users (id, email, role) VALUES (1, 'rina@x.co', 'tl');
         INSERT INTO projects (id, owner_id, name, slug) VALUES (1, 1, 'S', 'silent'), (2, 1, 'H', 'hooked'), (3, 1, 'K', 'kept');
         INSERT INTO alert_routes (project_id, severity, emails, webhook_url, use_webhook) VALUES
           (1, 'critical', '', NULL, 0), (1, 'warning', '', NULL, 0),
           (2, 'critical', '', 'https://hooks.example.com/x', 1), (2, 'warning', '', 'https://hooks.example.com/x', 1),
           (3, 'critical', 'oncall@x.co', NULL, 0), (3, 'warning', 'oncall@x.co', NULL, 0);",
    )
    .await;
    apply(&db, 22..23).await;
    assert_eq!(
        strings(
            &db,
            "SELECT emails FROM alert_routes ORDER BY project_id, severity"
        )
        .await,
        [
            "rina@x.co",
            "rina@x.co",
            "",
            "",
            "oncall@x.co",
            "oncall@x.co"
        ]
    );
}

/// Migration 33: a ticked webhook with no URL is unticked; a ticked webhook with a URL stays on.
#[tokio::test]
async fn migration_33_unticks_webhooks_without_a_url() {
    let (_dir, db) = at(32).await;
    exec(
        &db,
        "INSERT INTO users (id, email, role) VALUES (1, 'rina@x.co', 'tl');
         INSERT INTO projects (id, owner_id, name, slug) VALUES (1, 1, 'P', 'p');
         INSERT INTO alert_routes (project_id, severity, webhook_url, use_webhook) VALUES
           (1, 'critical', NULL, 1), (1, 'warning', 'https://hooks.example.com/x', 1);",
    )
    .await;
    apply(&db, 32..33).await;
    assert_eq!(
        strings(
            &db,
            "SELECT severity || ' ' || use_webhook FROM alert_routes ORDER BY severity"
        )
        .await,
        ["critical 0", "warning 1"]
    );
}

/// Migration 36 rebuilt alert_routes and outbox for Monita's Telegram bot; rows keep their ids.
#[tokio::test]
async fn migration_36_keeps_routes_and_outbox_rows_and_takes_the_bot() {
    let (_dir, db) = at(35).await;
    exec(
        &db,
        "INSERT INTO users (id, email, role) VALUES (1, 'a@x.co', 'tl');
         INSERT INTO projects (id, owner_id, name, slug) VALUES (1, 1, 'P', 'p');
         INSERT INTO alert_routes (id, project_id, severity, emails, webhook_url, use_webhook, enabled, webhook_format, webhook_key)
           VALUES (7, 1, 'critical', 'a@x.co', 'https://api.telegram.org/bot1:x/sendMessage', 1, 0, 'telegram', '-100');
         INSERT INTO outbox (id, channel, recipient, body, next_attempt_at, sent_at) VALUES (9, 'webhook', 'https://hook', '{}', 5, 6);",
    )
    .await;
    apply(&db, 35..36).await;
    assert_eq!(
        strings(
            &db,
            "SELECT id || ' ' || emails || ' ' || use_webhook || ' ' || enabled || ' ' || webhook_format || ' ' || webhook_key FROM alert_routes"
        )
        .await,
        ["7 a@x.co 1 0 telegram -100"]
    );
    assert_eq!(
        strings(&db, "SELECT CAST(sent_at AS TEXT) FROM outbox WHERE id = 9").await,
        ["6"]
    );
    exec(
        &db,
        "UPDATE alert_routes SET webhook_format = 'telegram_bot';
         INSERT INTO outbox (channel, recipient, body, next_attempt_at) VALUES ('telegram', '-100', '{}', 0);",
    )
    .await;
}

/// Migration 37 turns each project's two routes into alert cards: the owner's two, an Email card
/// for anyone else, a card per webhook. Nobody who got alerts stops getting them.
#[tokio::test]
async fn migration_37_turns_routes_into_cards() {
    let (_dir, db) = at(36).await;
    exec(
        &db,
        "INSERT INTO users (id, email, role) VALUES (1, 'own@x.co', 'tl');
         INSERT INTO projects (id, owner_id, name, slug) VALUES
           (1, 1, 'Only', 'only'), (2, 1, 'Others', 'others'), (3, 1, 'Look', 'look'), (4, 1, 'Hooks', 'hooks'), (5, 1, 'Bare', 'bare');
         INSERT INTO alert_routes (project_id, severity, emails, webhook_url, use_webhook, enabled, webhook_format, webhook_key) VALUES
           (1, 'critical', 'own@x.co', NULL, 0, 1, 'json', NULL),
           (1, 'warning', 'own@x.co', NULL, 0, 1, 'json', NULL),
           (2, 'critical', 'own@x.co' || char(10) || 'oncall@x.co', NULL, 0, 1, 'json', NULL),
           (2, 'warning', 'a@x.co, b@x.co', NULL, 0, 1, 'json', NULL),
           (3, 'critical', 'aown@x.co', NULL, 0, 1, 'json', NULL),
           (4, 'critical', '', 'https://discord.com/api/webhooks/1/x', 0, 1, 'discord', NULL),
           (4, 'warning', 'own@x.co', 'https://api.telegram.org/', 1, 0, 'telegram_bot', '-100');",
    )
    .await;
    apply(&db, 36..37).await;
    let cards = strings(
        &db,
        "SELECT project_id || ' ' || kind || ' c' || critical || ' w' || warning || ' on' || enabled
           || ' [' || replace(emails, char(10), ',') || '] ' || coalesce(url, '-') || ' ' || coalesce(key, '-')
         FROM alert_cards ORDER BY project_id, kind <> 'owner_email', id",
    )
    .await;
    assert_eq!(
        cards,
        [
            "1 owner_email c1 w0 on1 [] - -",
            "1 owner_email c0 w1 on1 [] - -",
            "2 owner_email c1 w0 on1 [] - -",
            "2 owner_email c0 w1 on0 [] - -",
            "2 email c1 w0 on1 [own@x.co,oncall@x.co] - -",
            "2 email c0 w1 on1 [a@x.co,b@x.co] - -",
            "3 owner_email c1 w0 on0 [] - -",
            "3 owner_email c0 w1 on1 [] - -",
            "3 email c1 w0 on1 [aown@x.co] - -",
            "4 owner_email c1 w0 on0 [] - -",
            "4 owner_email c0 w1 on0 [] - -",
            "4 discord c1 w0 on0 [] https://discord.com/api/webhooks/1/x -",
            "4 telegram_bot c0 w1 on0 [] https://api.telegram.org/ -100",
            "5 owner_email c1 w0 on1 [] - -",
            "5 owner_email c0 w1 on1 [] - -",
        ]
    );
    assert!(
        strings(
            &db,
            "SELECT name FROM sqlite_master WHERE name = 'alert_routes'"
        )
        .await
        .is_empty()
    );
    exec(
        &db,
        "INSERT INTO telegram_links (code_hash, project_id, expires_at) VALUES ('h', 1, 0);",
    )
    .await;
}
