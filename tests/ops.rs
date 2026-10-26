//! US-31 to US-34: rate limits, retention, backup, the peer ping.

mod common;

use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use axum::{Router, extract::State, routing::get};
use common::TestApp;
use monita::platform::{db::query_i64, now, settings};
use serde_json::Value;

async fn count(app: &TestApp, sql: &str) -> i64 {
    query_i64(&app.state.db.read().await.unwrap(), sql, ())
        .await
        .unwrap()
        .unwrap_or(0)
}

async fn set(app: &TestApp, key: &str, value: i64) {
    let key = key.to_string();
    app.state
        .db
        .write(async |c| settings::set(c, &key, value).await)
        .await
        .unwrap();
}

#[tokio::test]
async fn public_pages_are_limited_per_ip_and_health_is_not() {
    // AC-31.01, AC-31.03
    let app = TestApp::spawn("test").await;
    set(&app, "rl_ip_per_min", 3).await;
    for _ in 0..3 {
        let res = app.get("/api/ui/public/nope/status", None).await;
        assert_eq!(res.status(), 404);
    }
    let limited = app.get("/status/nope", None).await;
    assert_eq!(limited.status(), 429);
    assert!(limited.headers().contains_key("retry-after"));
    let api = app.get("/api/v1/public/status/nope", None).await;
    assert_eq!(api.status(), 429);
    assert_eq!(
        api.headers()["access-control-allow-origin"],
        "*",
        "a browser caller can read the 429"
    );
    let body: Value = api.json().await.unwrap();
    assert_eq!(body["error"]["code"], "rate_limited");
    assert_eq!(
        app.get("/healthz", None).await.status(),
        200,
        "health is never limited"
    );
}

#[tokio::test]
async fn an_api_key_is_limited_per_minute() {
    // AC-31.02
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    app.project(&tl, "P", "pp").await;
    let key =
        monita::projects::create_key(&app.state.db, 1, "k", monita::projects::KeyScope::RefIds)
            .await
            .unwrap();
    set(&app, "rl_key_per_min", 2).await;
    let call = || {
        app.http
            .post(app.url("/api/v1/ref-ids"))
            .bearer_auth(&key)
            .send()
    };
    assert_eq!(call().await.unwrap().status(), 201);
    let lower = app
        .http
        .post(app.url("/api/v1/ref-ids"))
        .header("authorization", format!("bearer {key}"))
        .send()
        .await
        .unwrap();
    assert_eq!(lower.status(), 201, "the scheme is case-insensitive");
    let third = call().await.unwrap();
    assert_eq!(third.status(), 429);
    let body: Value = third.json().await.unwrap();
    assert_eq!(body["error"]["code"], "rate_limited");
}

#[tokio::test]
async fn rollup_summarizes_old_days_before_the_sweep_deletes_raw_rows() {
    // AC-32.01, AC-32.02
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    app.project(&tl, "P", "pp").await;
    let t = now();
    let today = t - t.rem_euclid(86_400);
    let old_day = today - 8 * 86_400;
    app.state
        .db
        .write(async |c| {
            // `last_rollup_day` holds a date, as the job runner writes it. Any date works: the backfill
            // must read only `rollup_through`, and the old shared key made it parse this and stop.
            c.execute_batch(&format!(
                "INSERT INTO monitors (id, project_id, name, url, next_check_at) VALUES (1, 1, 'API', 'https://x.invalid/', 9999999999);
                 INSERT INTO check_results (monitor_id, checked_at, ok, latency_ms) VALUES (1, {a}, 1, 100), (1, {b}, 0, 300), (1, {t}, 1, 50);
                 INSERT INTO incidents (id, project_id, origin, title, severity, started_at, resolved_at) VALUES (1, 1, 'auto', 'API is down', 'warning', {a}, {c});
                 INSERT INTO incident_items VALUES (1, 'monitor', 1);
                 INSERT INTO sessions (user_id, token_hash, expires_at) VALUES (1, 'gone', 5);
                 INSERT INTO settings (key, value) VALUES ('rollup_through', '{since}'), ('last_rollup_day', '2026-10-16') ON CONFLICT (key) DO UPDATE SET value = excluded.value;",
                a = old_day + 3_600,
                b = old_day + 7_200,
                c = old_day + 3_600 + 900,
                since = today - 10 * 86_400,
            ))
            .await?;
            Ok(())
        })
        .await
        .unwrap();

    monita::checks::retention::run(&app.state.db, t)
        .await
        .unwrap();

    let day = monita::platform::time::day(old_day);
    let summary = |col: &str| {
        format!(
            "SELECT {col} FROM daily_summaries WHERE source_kind = 'monitor' AND source_id = 1 AND day = '{day}'"
        )
    };
    assert_eq!(count(&app, &summary("checks_total")).await, 2);
    assert_eq!(count(&app, &summary("checks_ok")).await, 1);
    assert_eq!(count(&app, &summary("avg_latency_ms")).await, 200);
    assert_eq!(count(&app, &summary("downtime_s")).await, 900);
    assert_eq!(
        count(&app, "SELECT count(*) FROM check_results").await,
        1,
        "only the row from today stays"
    );
    assert_eq!(
        count(&app, "SELECT count(*) FROM incidents").await,
        1,
        "incidents stay"
    );
    assert_eq!(
        count(
            &app,
            "SELECT count(*) FROM sessions WHERE token_hash = 'gone'"
        )
        .await,
        0
    );
    let yesterday = monita::platform::time::day(today - 86_400);
    assert_eq!(
        count(
            &app,
            &format!("SELECT count(*) FROM daily_summaries WHERE day = '{yesterday}'")
        )
        .await,
        1,
        "backfilled to yesterday"
    );
}

#[tokio::test]
async fn the_sweep_deletes_more_than_one_chunk_of_old_rows() {
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    app.project(&tl, "P", "pp").await;
    let t = now();
    app.state
        .db
        .write(async |c| {
            c.execute(
                "INSERT INTO monitors (id, project_id, name, url, next_check_at) VALUES (1, 1, 'API', 'https://x.invalid/', 9999999999)",
                (),
            )
            .await?;
            for i in 0..12_000 {
                c.execute(
                    "INSERT INTO check_results (monitor_id, checked_at, ok) VALUES (1, ?1, 1)",
                    [t - 30 * 86_400 + i],
                )
                .await?;
            }
            Ok(())
        })
        .await
        .unwrap();
    monita::checks::retention::run(&app.state.db, t)
        .await
        .unwrap();
    assert_eq!(count(&app, "SELECT count(*) FROM check_results").await, 0);
}

#[tokio::test]
async fn nightly_backup_is_a_readable_copy_and_old_ones_are_pruned() {
    // AC-33.01
    let dir = tempfile::tempdir().unwrap();
    let app = TestApp::spawn_with(
        "test",
        &[
            ("MONITA_BACKUP_DIR", dir.path().to_str().unwrap()),
            ("MONITA_BACKUP_KEEP", "2"),
        ],
    )
    .await;
    app.tl("rina@x.co").await;
    for old in ["monita-20200101.db", "monita-20200102.db"] {
        std::fs::write(dir.path().join(old), b"old").unwrap();
    }
    let file = monita::platform::backup::run(&app.state, now())
        .await
        .unwrap()
        .unwrap();
    assert!(file.exists());
    let copy = monita::platform::db::Db::open(&file).await.unwrap();
    let users = query_i64(
        &copy.read().await.unwrap(),
        "SELECT count(*) FROM users",
        (),
    )
    .await
    .unwrap();
    assert_eq!(users, Some(1));
    // Opening the copy above creates its WAL; count only the backups themselves.
    let mut names: Vec<String> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .filter(|n| n.ends_with(".db"))
        .collect();
    names.sort();
    assert_eq!(names.len(), 2, "{names:?}");
    assert_eq!(names[0], "monita-20200102.db", "the oldest is pruned");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&file).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    // A second run the same day replaces the file instead of failing.
    assert!(
        monita::platform::backup::run(&app.state, now())
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        !dir.path().join(".partial").exists(),
        "the private staging folder is gone"
    );
}

async fn setting(app: &TestApp, key: &str) -> Option<String> {
    settings::get_raw(&app.state.db.read().await.unwrap(), key)
        .await
        .unwrap()
}

#[tokio::test]
async fn a_failed_backup_does_not_stop_the_digest_and_waits_before_retrying() {
    let dir = tempfile::tempdir().unwrap();
    let not_a_dir = dir.path().join("file");
    std::fs::write(&not_a_dir, b"x").unwrap();
    let app = TestApp::spawn_with(
        "test",
        &[("MONITA_BACKUP_DIR", not_a_dir.to_str().unwrap())],
    )
    .await;
    // Tomorrow 23:00 UTC: every job is due, and the real scheduler is still on today.
    let t = now() - now().rem_euclid(86_400) + 86_400 + 23 * 3_600;
    let day = monita::platform::time::day(t);
    monita::platform::jobs::run_due(&app.state, t).await;
    assert_eq!(
        setting(&app, "last_digest_day").await.as_deref(),
        Some(day.as_str())
    );
    assert_ne!(
        setting(&app, "last_backup_day").await.as_deref(),
        Some(day.as_str())
    );
    assert_eq!(
        setting(&app, "last_backup_attempt_at").await,
        Some(t.to_string())
    );

    monita::platform::jobs::run_due(&app.state, t + 60).await;
    assert_eq!(
        setting(&app, "last_backup_attempt_at").await,
        Some(t.to_string()),
        "the next tick does not retry yet"
    );
}

#[tokio::test]
async fn monita_pings_its_peer_at_start_and_every_minute() {
    // AC-34.03
    let hits = Arc::new(AtomicUsize::new(0));
    let peer = Router::new()
        .route(
            "/ping/peer",
            get(|State(h): State<Arc<AtomicUsize>>| async move {
                h.fetch_add(1, Ordering::SeqCst);
                "OK"
            }),
        )
        .with_state(hits.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, peer).await.unwrap() });
    let _app = TestApp::spawn_with(
        "test",
        &[("MONITA_PING_URL", &format!("http://{addr}/ping/peer"))],
    )
    .await;
    for _ in 0..30 {
        if hits.load(Ordering::SeqCst) > 0 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert_eq!(hits.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn wrong_api_keys_are_limited_per_ip() {
    let app = TestApp::spawn("test").await;
    set(&app, "rl_ip_per_min", 2).await;
    let call = || {
        app.http
            .post(app.url("/api/v1/ref-ids"))
            .bearer_auth("mk_aaaaaaaaaaaaaaaaaaaaaaaaaa")
            .send()
    };
    for _ in 0..2 {
        assert_eq!(call().await.unwrap().status(), 401);
    }
    let limited = call().await.unwrap();
    assert_eq!(limited.status(), 429);
    assert!(limited.headers().contains_key("retry-after"));
}

#[tokio::test]
async fn invite_links_are_limited_per_ip_when_read() {
    let app = TestApp::spawn("test").await;
    set(&app, "rl_ip_per_min", 2).await;
    for _ in 0..2 {
        let res = app.get("/api/ui/invites/not-a-token", None).await;
        assert_eq!(res.status(), 410);
    }
    let res = app.get("/api/ui/invites/not-a-token", None).await;
    assert_eq!(res.status(), 429);
}

#[tokio::test]
async fn status_page_traffic_does_not_use_up_sign_in() {
    let app = TestApp::spawn("test").await;
    set(&app, "rl_ip_per_min", 2).await;
    for _ in 0..3 {
        app.get("/api/ui/public/nope/status", None).await;
    }
    let login = || {
        app.api_post(
            "/api/ui/login",
            serde_json::json!({ "email": "rina@x.co", "password": "wrong-password" }),
            None,
        )
    };
    for _ in 0..2 {
        assert_ne!(login().await.status(), 429, "sign-in has its own bucket");
    }
    assert_eq!(login().await.status(), 429, "and its own limit");
}

/// SEC-18: the database holds secrets, so it is private even when the umask is not.
#[cfg(unix)]
#[tokio::test]
async fn the_database_files_are_readable_by_their_owner_only() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("monita.db");
    let db = monita::platform::db::Db::open(&path).await.unwrap();
    db.write(async |c| {
        c.execute("CREATE TABLE t (x INTEGER)", ()).await?;
        Ok(())
    })
    .await
    .unwrap();
    // The WAL exists from the first connection, so open covers it.
    for entry in std::fs::read_dir(dir.path()).unwrap() {
        let f = entry.unwrap();
        let mode = f.metadata().unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "{:?}", f.file_name());
    }
}
