//! Hard caps (src/platform/caps.rs, spec 08 section 8.2): a capped list refuses one more row at
//! insert, and every job works through a backlog in batches.

mod common;

use common::TestApp;
use monita::platform::{
    caps,
    db::{query_all, query_i64},
    now,
};

/// Runs `sql` as one batch on the writer.
async fn sql(app: &TestApp, sql: &str) {
    let sql = sql.to_string();
    app.state
        .db
        .write(async |c| {
            c.execute_batch(&sql).await?;
            Ok(())
        })
        .await
        .unwrap();
}

async fn count(app: &TestApp, q: &str) -> i64 {
    query_i64(&app.state.db.read().await.unwrap(), q, ())
        .await
        .unwrap()
        .unwrap_or(0)
}

/// `n` rows from a recursive counter `n(i)`, i = 1..=n, for `INSERT ... SELECT ... FROM n`.
fn upto(n: i64) -> String {
    format!("WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < {n})")
}

/// A TL with the project `payments`. Returns the TL's cookie.
async fn tl_with_project(app: &TestApp) -> String {
    let tl = app.tl("rina@x.co").await;
    let res = app.project(&tl, "Payments", "payments").await;
    assert_eq!(res.status(), 303);
    tl
}

#[tokio::test]
async fn a_full_instance_refuses_another_project() {
    let app = TestApp::spawn("test").await;
    let tl = tl_with_project(&app).await;
    let more = caps::PROJECTS - 1;
    sql(
        &app,
        &format!(
            "INSERT INTO projects (owner_id, name, slug) {} SELECT 1, 'P' || i, 'p-' || i FROM n;",
            upto(more)
        ),
    )
    .await;
    let res = app.project(&tl, "One more", "one-more").await;
    assert_eq!(res.status(), 422);
    assert!(res.text().await.unwrap().contains("200 projects"));
    assert_eq!(
        count(&app, "SELECT count(*) FROM projects").await,
        caps::PROJECTS
    );
}

#[tokio::test]
async fn a_full_project_refuses_another_monitor_and_heartbeat() {
    let app = TestApp::spawn("test").await;
    let tl = tl_with_project(&app).await;
    sql(&app, &format!(
        "INSERT INTO monitors (project_id, name, url, next_check_at) {0} SELECT 1, 'm' || i, 'https://x.co', 9999999999 FROM n;
         INSERT INTO heartbeats (project_id, name, token, period_s, grace_s) {0} SELECT 1, 'h' || i, 'tok-' || i, 3600, 0 FROM n;",
        upto(caps::MONITORS_PER_PROJECT)
    ))
    .await;
    let res = app
        .post_form(
            "/projects/payments/monitors",
            &[
                ("name", "API"),
                ("url", "https://api.x.co"),
                ("interval_s", "60"),
                ("timeout_s", "10"),
                ("expected_status", "2xx"),
                ("keyword", ""),
                ("severity", "warning"),
            ],
            Some(&tl),
        )
        .await;
    assert_eq!(res.status(), 422);
    assert!(res.text().await.unwrap().contains(caps::MONITORS_FULL));
    let res = app
        .post_form(
            "/projects/payments/heartbeats",
            &[
                ("name", "Backup"),
                ("period_s", "86400"),
                ("grace_s", "1800"),
                ("severity", "warning"),
            ],
            Some(&tl),
        )
        .await;
    assert_eq!(res.status(), 422);
    assert!(res.text().await.unwrap().contains(caps::HEARTBEATS_FULL));
}

#[tokio::test]
async fn a_project_holds_20_live_keys_and_a_revoked_one_frees_a_slot() {
    let app = TestApp::spawn("test").await;
    let tl = tl_with_project(&app).await;
    sql(&app, &format!("INSERT INTO api_keys (project_id, name, key_hash, key_prefix) {} SELECT 1, 'k' || i, 'hash-' || i, 'mk_x' FROM n;", upto(caps::KEYS_PER_PROJECT))).await;
    let res = app
        .post_form(
            "/projects/payments/keys",
            &[("name", "one more")],
            Some(&tl),
        )
        .await;
    assert_eq!(res.status(), 422);
    assert!(res.text().await.unwrap().contains("20 live keys"));
    sql(
        &app,
        "UPDATE api_keys SET revoked_at = 1 WHERE name = 'k1';",
    )
    .await;
    let res = app
        .post_form(
            "/projects/payments/keys",
            &[("name", "one more")],
            Some(&tl),
        )
        .await;
    assert_eq!(res.status(), 200);
}

#[tokio::test]
async fn a_full_instance_refuses_another_account() {
    let app = TestApp::spawn("test").await;
    sql(
        &app,
        &format!(
            "INSERT INTO users (email, role) {} SELECT 'u' || i || '@x.co', 'tl' FROM n;",
            upto(caps::USERS)
        ),
    )
    .await;
    let err = monita::accounts::invite_tl(&app.state.db, common::BASE_URL, "late@x.co")
        .await
        .unwrap_err();
    assert!(matches!(err, monita::accounts::AccountError::Full), "{err}");
}

#[tokio::test]
async fn an_incident_takes_50_updates_and_a_project_20_open_manual_incidents() {
    let app = TestApp::spawn("test").await;
    tl_with_project(&app).await;
    sql(&app, &format!(
        "INSERT INTO monitors (project_id, name, url, next_check_at) VALUES (1, 'API', 'https://x.co', 9999999999);
         INSERT INTO incidents (project_id, origin, title, severity, started_at) {0} SELECT 1, 'manual', 'i' || i, 'warning', 1 FROM n;
         INSERT INTO incident_items (incident_id, source_kind, source_id) SELECT id, 'monitor', 1 FROM incidents;",
        upto(caps::OPEN_MANUAL_PER_PROJECT)
    ))
    .await;
    sql(&app, &format!("INSERT INTO status_updates (incident_id, label, message) {} SELECT 1, 'investigating', 'u' || i FROM n;", upto(caps::UPDATES_PER_INCIDENT))).await;
    let err = monita::incidents::post_update(&app.state.db, 1, 1, "identified", "found", 1, now())
        .await
        .unwrap_err();
    assert!(
        matches!(err, monita::incidents::UpdateError::TooManyUpdates),
        "{err}"
    );
    let new = monita::incidents::NewIncident {
        project_id: 1,
        title: "One more".into(),
        items: vec!["monitor:1".into()],
        severity: "warning".into(),
        message: "Looking.".into(),
        author: 1,
    };
    let err = monita::incidents::open_manual(&app.state.db, &new, now())
        .await
        .unwrap_err();
    assert!(
        matches!(err, monita::incidents::UpdateError::TooManyOpen),
        "{err}"
    );
}

#[tokio::test]
async fn an_alert_route_takes_20_addresses() {
    let app = TestApp::spawn("test").await;
    tl_with_project(&app).await;
    let emails: Vec<String> = (0..=caps::EMAILS_PER_ROUTE)
        .map(|i| format!("a{i}@x.co"))
        .collect();
    let err =
        monita::notify::save_route(&app.state.db, 1, "critical", &emails.join("\n"), "", false)
            .await
            .unwrap_err();
    assert!(
        matches!(err, monita::notify::RouteError::TooManyEmails),
        "{err}"
    );
    monita::notify::save_route(
        &app.state.db,
        1,
        "critical",
        &emails[1..].join("\n"),
        "",
        false,
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn a_board_column_shows_its_newest_200_and_counts_them_all() {
    let app = TestApp::spawn("test").await;
    let tl = tl_with_project(&app).await;
    let n = caps::TICKETS_PER_COLUMN + 1;
    sql(&app, &format!(
        "INSERT INTO ref_ids (project_id, token_hash) VALUES (1, 'ref');
         INSERT INTO tickets (project_id, number, ref_id, title, description, stage, created_at, updated_at) {} SELECT 1, i, 1, 'bug ' || i, 'd', 'backlog', i, i FROM n;",
        upto(n)
    ))
    .await;
    let html = app
        .get("/projects/payments/tickets", Some(&tl))
        .await
        .text()
        .await
        .unwrap();
    assert_eq!(
        html.matches(r#"class="card""#).count() as i64,
        caps::TICKETS_PER_COLUMN
    );
    assert!(html.contains(&format!(r#"<span class="board__count">{n}</span>"#)));
    assert!(
        !html.contains(">bug 1<"),
        "the oldest ticket is the one left out"
    );
}

#[tokio::test]
async fn the_ping_list_pages_with_an_older_link() {
    let app = TestApp::spawn("test").await;
    let tl = tl_with_project(&app).await;
    let t = now();
    sql(&app, &format!(
        "INSERT INTO heartbeats (project_id, name, token, period_s, grace_s) VALUES (1, 'Backup', 'tok', 3600, 0);
         INSERT INTO pings (heartbeat_id, received_at, kind) {} SELECT 1, {t} - i, 'ok' FROM n;",
        upto(caps::PINGS_PAGE + 1)
    ))
    .await;
    let first = app
        .get("/projects/payments/heartbeats/1", Some(&tl))
        .await
        .text()
        .await
        .unwrap();
    let before = first
        .split("&amp;before=")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .expect("an Older pings link");
    let second = app
        .get(
            &format!("/projects/payments/heartbeats/1?before={before}"),
            Some(&tl),
        )
        .await
        .text()
        .await
        .unwrap();
    assert!(
        !second.contains("&amp;before="),
        "the last page has no Older link"
    );
    // Each ping row carries an OK state; nothing else on the page does.
    let rows = |html: &str| html.matches("state--up\">OK</span>").count();
    assert_eq!(
        rows(&first) as i64 + rows(&second) as i64,
        caps::PINGS_PAGE + 1
    );
}

#[tokio::test]
async fn the_missed_sweep_works_through_a_backlog_in_batches() {
    let app = TestApp::spawn("test").await;
    tl_with_project(&app).await;
    let n = caps::MISSED_BATCH * 2 + 3;
    sql(&app, &format!("INSERT INTO heartbeats (project_id, name, token, period_s, grace_s, deadline_at) {} SELECT 1, 'h' || i, 'tok-' || i, 3600, 0, 1 FROM n;", upto(n))).await;
    let read = app.state.db.read().await.unwrap();
    let swept =
        monita::checks::heartbeat::sweep_missed(&app.state.db, &read, common::BASE_URL, now())
            .await
            .unwrap();
    assert_eq!(swept as i64, n);
    assert_eq!(count(&app, "SELECT count(*) FROM incidents").await, n);
    assert_eq!(
        count(
            &app,
            "SELECT count(*) FROM heartbeats WHERE deadline_at IS NOT NULL"
        )
        .await,
        0
    );
}

#[tokio::test]
async fn retention_deletes_a_large_outbox_backlog_in_batches() {
    let app = TestApp::spawn("test").await;
    let n = caps::JOB_BATCH * 2 + 7;
    sql(&app, &format!("INSERT INTO outbox (channel, recipient, body, next_attempt_at, sent_at, created_at) {} SELECT 'email', 'a@x.co', '', 0, 1, 1 FROM n;", upto(n))).await;
    monita::checks::retention::run(&app.state.db, now())
        .await
        .unwrap();
    assert_eq!(count(&app, "SELECT count(*) FROM outbox").await, 0);
}

#[tokio::test]
async fn a_digest_lists_50_tickets_and_says_how_many_more_wait() {
    let app = TestApp::spawn("test").await;
    tl_with_project(&app).await;
    let n = caps::DIGEST_LINES + 10;
    sql(&app, &format!(
        "INSERT INTO ref_ids (project_id, token_hash) VALUES (1, 'ref');
         INSERT INTO tickets (project_id, number, ref_id, title, description) {} SELECT 1, i, 1, 'bug ' || i, 'd' FROM n;",
        upto(n)
    ))
    .await;
    assert_eq!(
        monita::notify::digest(&app.state.db, common::BASE_URL, now())
            .await
            .unwrap(),
        1
    );
    let (subject, body): (String, String) = query_all(
        &app.state.db.read().await.unwrap(),
        "SELECT subject, body FROM outbox WHERE subject LIKE '[Monita]%'",
        (),
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .await
    .unwrap()
    .pop()
    .unwrap();
    assert!(subject.ends_with(&format!("{n} new tickets")), "{subject}");
    assert_eq!(
        body.matches("\n#").count() as i64 + i64::from(body.starts_with('#')),
        caps::DIGEST_LINES
    );
    assert!(body.contains("and 10 more on the board"), "{body}");
}

#[tokio::test]
async fn the_rollup_summarizes_every_source_past_one_batch() {
    let app = TestApp::spawn("test").await;
    tl_with_project(&app).await;
    let n = caps::JOB_BATCH + 3;
    // The app's own start already rolled up to yesterday; leave one day to do.
    let two_days_ago = monita::platform::time::day_start(now()) - 2 * 86_400;
    sql(&app, &format!(
        "INSERT INTO monitors (project_id, name, url, next_check_at) {} SELECT 1, 'm' || i, 'https://x.co', 9999999999 FROM n;
         INSERT INTO settings (key, value) VALUES ('rollup_through', '{two_days_ago}') ON CONFLICT (key) DO UPDATE SET value = excluded.value;",
        upto(n)
    ))
    .await;
    monita::checks::retention::run(&app.state.db, now())
        .await
        .unwrap();
    assert_eq!(
        count(
            &app,
            "SELECT count(*) FROM daily_summaries WHERE source_kind = 'monitor'"
        )
        .await,
        n
    );
}
