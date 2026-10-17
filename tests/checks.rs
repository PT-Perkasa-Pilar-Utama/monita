//! US-09 to US-11. The target is a local axum server; the clock is moved through next_check_at.

mod common;

use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use axum::{Router, http::StatusCode, routing::get};
use common::{TestApp, location};
use monita::{
    checks::{self, Clients, MonitorInput, probe},
    platform::db::query_i64,
};

/// `/ok` 200 "all good", `/fail` 503, `/slow` waits 600 ms. Counts requests in flight.
async fn target() -> (String, Arc<AtomicUsize>) {
    let in_flight = Arc::new(AtomicUsize::new(0));
    let peak = Arc::new(AtomicUsize::new(0));
    let (f, p) = (in_flight.clone(), peak.clone());
    let app = Router::new()
        .route("/ok", get(|| async { "all good" }))
        .route(
            "/moved",
            get(|| async { (StatusCode::MOVED_PERMANENTLY, [("location", "/ok")]) }),
        )
        .route(
            "/fail",
            get(|| async { (StatusCode::SERVICE_UNAVAILABLE, "down") }),
        )
        .route(
            "/slow",
            get(move || {
                let (f, p) = (f.clone(), p.clone());
                async move {
                    let now = f.fetch_add(1, Ordering::SeqCst) + 1;
                    p.fetch_max(now, Ordering::SeqCst);
                    tokio::time::sleep(Duration::from_millis(600)).await;
                    f.fetch_sub(1, Ordering::SeqCst);
                    "slow"
                }
            }),
        );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (format!("http://{addr}"), peak)
}

fn input(name: &str, url: &str) -> MonitorInput {
    MonitorInput {
        name: name.into(),
        url: url.into(),
        ..MonitorInput::defaults()
    }
}

/// Makes every monitor due now: the test stands in for the clock.
async fn make_due(app: &TestApp) {
    app.state
        .db
        .write(async |c| {
            c.execute("UPDATE monitors SET next_check_at = 0", ())
                .await?;
            Ok(())
        })
        .await
        .unwrap();
}

async fn count(app: &TestApp, sql: &str) -> i64 {
    query_i64(&app.state.db.read().await.unwrap(), sql, ())
        .await
        .unwrap()
        .unwrap_or(0)
}

async fn wait_for(app: &TestApp, sql: &str, at_least: i64) -> i64 {
    for _ in 0..50 {
        let n = count(app, sql).await;
        if n >= at_least {
            return n;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    count(app, sql).await
}

async fn tl_project(app: &TestApp) -> String {
    let tl = app.tl("rina@x.co").await;
    app.post_form(
        "/projects",
        &[("name", "Payments"), ("slug", "payments")],
        Some(&tl),
    )
    .await;
    tl
}

#[tokio::test]
async fn probe_judges_status_keyword_and_timeout() {
    let (base, _) = target().await;
    let clients = Clients::new().unwrap();
    let run = |url: String, keyword: Option<&'static str>, timeout_ms: i64| {
        let clients = clients.clone();
        async move {
            let t = checks::Target {
                url: &url,
                timeout_ms,
                expected_status: "2xx",
                keyword,
                skip_tls_verify: false,
            };
            probe(&t, &clients).await
        }
    };
    let ok = run(format!("{base}/ok"), Some("good"), 2000).await;
    assert!(ok.ok);
    assert_eq!(ok.status_code, Some(200));

    let fail = run(format!("{base}/fail"), None, 2000).await;
    assert!(!fail.ok);
    assert_eq!(fail.error.as_deref(), Some("HTTP 503, expected 2xx"));

    let missing = run(format!("{base}/ok"), Some("database"), 2000).await;
    assert!(!missing.ok);
    assert_eq!(
        missing.error.as_deref(),
        Some("Keyword \"database\" not found")
    );

    let slow = run(format!("{base}/slow"), None, 1000).await;
    assert!(slow.ok);
    let timeout = run(format!("{base}/slow"), None, 200).await;
    assert_eq!(timeout.error.as_deref(), Some("Timeout after 200 ms"));

    let refused = run("http://127.0.0.1:9/".into(), None, 2000).await;
    assert!(!refused.ok);
    assert!(refused.error.unwrap().starts_with("Connection failed"));
}

#[tokio::test]
async fn a_monitor_that_expects_a_redirect_sees_it_and_others_follow_it() {
    let (base, _) = target().await;
    let clients = Clients::new().unwrap();
    let url = format!("{base}/moved");
    let check = |expected_status| checks::Target {
        url: &url,
        timeout_ms: 2000,
        expected_status,
        keyword: None,
        skip_tls_verify: false,
    };
    let seen = probe(&check("301"), &clients).await;
    assert!(seen.ok, "{:?}", seen.error);
    assert_eq!(seen.status_code, Some(301));
    let followed = probe(&check("2xx"), &clients).await;
    assert!(followed.ok);
    assert_eq!(followed.status_code, Some(200));
}

#[tokio::test]
async fn new_monitor_form_shows_every_field_with_defaults() {
    // AC-09.01, AC-09.02
    let app = TestApp::spawn("test").await;
    let tl = tl_project(&app).await;
    let html = app
        .get("/projects/payments/monitors/new", Some(&tl))
        .await
        .text()
        .await
        .unwrap();
    for field in [
        "name=\"name\"",
        "name=\"url\"",
        "name=\"interval_s\"",
        "name=\"timeout_s\"",
        "name=\"expected_status\"",
        "name=\"keyword\"",
        "name=\"severity\"",
        "name=\"is_public\"",
    ] {
        assert!(html.contains(field), "missing {field}");
    }
    assert!(html.contains("<option value=\"60\" selected>1m</option>"));
    assert!(html.contains("value=\"10\""), "timeout 10 s");
    assert!(html.contains("value=\"2xx\""));
    assert!(html.contains("value=\"warning\" checked"));
    assert!(!html.contains("name=\"is_public\" value=\"on\" checked"));
}

#[tokio::test]
async fn tl_creates_a_monitor_and_bad_input_is_refused() {
    let app = TestApp::spawn("test").await;
    let tl = tl_project(&app).await;
    let form = [
        ("name", "API health"),
        ("url", "https://pay.internal/health"),
        ("interval_s", "30"),
        ("timeout_s", "5"),
        ("expected_status", "200,204"),
        ("keyword", ""),
        ("severity", "critical"),
        ("is_public", "on"),
    ];
    let res = app
        .post_form("/projects/payments/monitors", &form, Some(&tl))
        .await;
    assert_eq!(res.status(), 303);
    let detail = app
        .get(&location(&res), Some(&tl))
        .await
        .text()
        .await
        .unwrap();
    assert!(
        detail.contains("API health") && detail.contains("200,204") && detail.contains("Critical")
    );

    let bad = [
        ("name", ""),
        ("url", "ftp://x"),
        ("interval_s", "45"),
        ("timeout_s", "99"),
        ("expected_status", "ok"),
        ("severity", "warning"),
    ];
    let res = app
        .post_form("/projects/payments/monitors", &bad, Some(&tl))
        .await;
    assert_eq!(res.status(), 422);
    let html = res.text().await.unwrap();
    assert!(html.contains("The URL must start with http:// or https://."));
    assert_eq!(count(&app, "SELECT count(*) FROM monitors").await, 1);
}

#[tokio::test]
async fn the_monitor_list_filters_by_search_and_severity() {
    let app = TestApp::spawn("test").await;
    let tl = tl_project(&app).await;
    for (name, severity) in [("Checkout API", "critical"), ("Admin panel", "warning")] {
        let url = format!("https://{}.internal/", name.len());
        let form = [
            ("name", name),
            ("url", url.as_str()),
            ("interval_s", "60"),
            ("timeout_s", "10"),
            ("expected_status", "2xx"),
            ("severity", severity),
        ];
        let res = app
            .post_form("/projects/payments/monitors", &form, Some(&tl))
            .await;
        assert_eq!(res.status(), 303);
    }
    let page = |q: &'static str| {
        let (app, tl) = (&app, &tl);
        async move {
            let path = format!("/projects/payments/monitors{q}");
            app.get(&path, Some(tl)).await.text().await.unwrap()
        }
    };
    let html = page("?q=checkout").await;
    assert!(html.contains("Checkout API") && !html.contains("Admin panel"));
    assert!(html.contains("1 of 2 monitors"));
    let html = page("?severity=warning").await;
    assert!(!html.contains("Checkout API") && html.contains("Admin panel"));
    let html = page("?q=nothing-like-this").await;
    assert!(html.contains("Nothing matches these filters"));
}

#[tokio::test]
async fn due_monitors_are_checked_and_the_detail_lists_the_results() {
    // AC-10.01
    let app = TestApp::spawn("test").await;
    let tl = tl_project(&app).await;
    let (base, _) = target().await;
    checks::create_monitor(&app.state.db, 1, &input("OK page", &format!("{base}/ok")))
        .await
        .unwrap();
    checks::create_monitor(
        &app.state.db,
        1,
        &input("Broken page", &format!("{base}/fail")),
    )
    .await
    .unwrap();
    make_due(&app).await;
    assert_eq!(
        wait_for(&app, "SELECT count(*) FROM check_results", 2).await,
        2
    );

    let html = app
        .get("/projects/payments/monitors/2", Some(&tl))
        .await
        .text()
        .await
        .unwrap();
    assert!(html.contains("503"));
    assert!(html.contains("HTTP 503, expected 2xx"));
    assert!(html.contains("ms</td>"), "latency column");
    let list = app
        .get("/projects/payments/monitors", Some(&tl))
        .await
        .text()
        .await
        .unwrap();
    assert!(
        !list.contains("state state--down"),
        "AC-17.02: one failed check is not an outage"
    );
    assert!(list.contains("200 in "));
}

#[tokio::test]
async fn a_checked_monitor_moves_one_interval_ahead() {
    let app = TestApp::spawn("test").await;
    tl_project(&app).await;
    let (base, _) = target().await;
    checks::create_monitor(&app.state.db, 1, &input("OK", &format!("{base}/ok")))
        .await
        .unwrap();
    make_due(&app).await;
    wait_for(&app, "SELECT count(*) FROM check_results", 1).await;
    let next = count(&app, "SELECT next_check_at FROM monitors").await;
    let now = monita::platform::now();
    assert!(
        next > now + 50 && next <= now + 60,
        "next {next}, now {now}"
    );
    tokio::time::sleep(Duration::from_millis(1500)).await;
    assert_eq!(
        count(&app, "SELECT count(*) FROM check_results").await,
        1,
        "not checked twice"
    );
}

#[tokio::test]
async fn new_monitors_start_at_random_offsets_inside_their_interval() {
    // AC-11.01
    let app = TestApp::spawn("test").await;
    tl_project(&app).await;
    let before = monita::platform::now();
    for i in 0..20 {
        let m = MonitorInput {
            interval_s: 300,
            ..input(&format!("m{i}"), "https://example.invalid/")
        };
        checks::create_monitor(&app.state.db, 1, &m).await.unwrap();
    }
    let distinct = count(&app, "SELECT count(DISTINCT next_check_at) FROM monitors").await;
    assert!(distinct >= 10, "only {distinct} distinct start times");
    let min = count(&app, "SELECT min(next_check_at) FROM monitors").await;
    let max = count(&app, "SELECT max(next_check_at) FROM monitors").await;
    assert!(min >= before && max < before + 301);
}

#[tokio::test]
async fn no_more_checks_run_at_once_than_the_cap() {
    // AC-11.02
    let app = TestApp::spawn("test").await;
    let admin = app.super_admin("admin@x.co").await;
    let mut form: Vec<(&str, &str)> = monita::platform::settings::KEYS
        .iter()
        .map(|(k, _)| (*k, "50"))
        .collect();
    form.retain(|(k, _)| *k != "check_concurrency");
    form.push(("check_concurrency", "2"));
    assert_eq!(
        app.post_form("/admin/settings", &form, Some(&admin))
            .await
            .status(),
        303
    );
    // The scheduler re-reads the cap on every tick.
    tokio::time::sleep(Duration::from_millis(1_500)).await;

    tl_project(&app).await;
    let (base, peak) = target().await;
    for i in 0..6 {
        checks::create_monitor(
            &app.state.db,
            1,
            &input(&format!("slow{i}"), &format!("{base}/slow")),
        )
        .await
        .unwrap();
    }
    make_due(&app).await;
    assert_eq!(
        wait_for(&app, "SELECT count(*) FROM check_results", 6).await,
        6
    );
    assert_eq!(peak.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn limits_page_is_for_super_admins_and_rejects_bad_values() {
    // AC-11.03
    let app = TestApp::spawn("test").await;
    let admin = app.super_admin("admin@x.co").await;
    let tl = app.tl("rina@x.co").await;
    assert_eq!(app.get("/admin/settings", Some(&tl)).await.status(), 403);
    let html = app
        .get("/admin/settings", Some(&admin))
        .await
        .text()
        .await
        .unwrap();
    assert!(html.contains("name=\"check_concurrency\"") && html.contains("value=\"50\""));
    let bad = app
        .post_form(
            "/admin/settings",
            &[("check_concurrency", "0")],
            Some(&admin),
        )
        .await;
    assert_eq!(bad.status(), 422);
    let mut huge: Vec<(&str, &str)> = monita::platform::settings::KEYS
        .iter()
        .map(|(k, _)| (*k, "50"))
        .collect();
    huge.retain(|(k, _)| *k != "rl_ip_per_min");
    huge.push(("rl_ip_per_min", "4294967296"));
    let res = app.post_form("/admin/settings", &huge, Some(&admin)).await;
    assert_eq!(res.status(), 422, "a value that would wrap a u32");
    let set = app
        .state
        .db
        .write(async |c| monita::platform::settings::set(c, "check_concurrency", 501).await)
        .await;
    assert!(set.is_err(), "check_concurrency stops at 500");
}

#[tokio::test]
async fn the_system_page_queues_a_test_email_to_the_signed_in_admin_only() {
    let app = TestApp::spawn("test").await;
    let admin = app.super_admin("admin@x.co").await;
    let tl = app.tl("rina@x.co").await;
    let none: &[(&str, &str)] = &[];
    let refused = app
        .post_form("/admin/settings/test-email", none, Some(&tl))
        .await;
    assert_eq!(refused.status(), 403);
    let res = app
        .post_form("/admin/settings/test-email", none, Some(&admin))
        .await;
    assert_eq!(res.status(), 303);
    let queued = monita::platform::db::query_i64(
        &app.state.db.read().await.unwrap(),
        "SELECT count(*) FROM outbox WHERE channel = 'email' AND recipient = 'admin@x.co' AND subject = 'Monita test email'",
        (),
    )
    .await
    .unwrap();
    assert_eq!(queued, Some(1));
    let html = app
        .get("/admin/settings?sent=1", Some(&admin))
        .await
        .text()
        .await
        .unwrap();
    assert!(html.contains("Test email queued for admin@x.co"));
}

#[tokio::test]
async fn a_blank_number_re_renders_the_form_with_its_error() {
    let app = TestApp::spawn("test").await;
    let tl = tl_project(&app).await;
    let res = app
        .post_form(
            "/projects/payments/monitors",
            &[
                ("name", "API"),
                ("url", "https://x.invalid/"),
                ("interval_s", "60"),
                ("timeout_s", ""),
                ("expected_status", "2xx"),
                ("severity", "warning"),
            ],
            Some(&tl),
        )
        .await;
    assert_eq!(res.status(), 422);
    let html = res.text().await.unwrap();
    assert!(html.contains("Use 1 to 30 seconds.") && html.contains("value=\"API\""));
}

#[tokio::test]
async fn a_passing_check_on_a_healthy_monitor_writes_no_incident_state() {
    let app = TestApp::spawn("test").await;
    tl_project(&app).await;
    let (base, _) = target().await;
    checks::create_monitor(&app.state.db, 1, &input("API", &format!("{base}/ok")))
        .await
        .unwrap();
    make_due(&app).await;
    assert_eq!(
        wait_for(&app, "SELECT count(*) FROM check_results", 1).await,
        1
    );
    assert_eq!(
        count(&app, "SELECT count(*) FROM incident_sources").await,
        0
    );
}

#[tokio::test]
async fn older_checks_keep_rows_that_share_a_second() {
    let app = TestApp::spawn("test").await;
    let tl = tl_project(&app).await;
    let t = monita::platform::now() - 60;
    app.state
        .db
        .write(async |c| {
            c.execute(
                "INSERT INTO monitors (id, project_id, name, url, next_check_at) VALUES (1, 1, 'API', 'https://x.invalid/', 9999999999)",
                (),
            )
            .await?;
            for _ in 0..26 {
                c.execute(
                    "INSERT INTO check_results (monitor_id, checked_at, ok, error) VALUES (1, ?1, 0, 'row')",
                    [t],
                )
                .await?;
            }
            Ok(())
        })
        .await
        .unwrap();
    let first = app
        .get("/projects/payments/monitors/1", Some(&tl))
        .await
        .text()
        .await
        .unwrap();
    assert_eq!(first.matches(">row<").count(), 25);
    let link = first
        .split("&amp;before=")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .expect("an older-checks link");
    let second = app
        .get(
            &format!("/projects/payments/monitors/1?before={link}"),
            Some(&tl),
        )
        .await
        .text()
        .await
        .unwrap();
    assert_eq!(
        second.matches(">row<").count(),
        1,
        "the 26th check is not skipped"
    );
}

#[tokio::test]
async fn another_project_cannot_open_a_monitor_by_id() {
    let app = TestApp::spawn("test").await;
    tl_project(&app).await;
    checks::create_monitor(&app.state.db, 1, &input("Secret", "https://x.invalid/"))
        .await
        .unwrap();
    let budi = app.tl("budi@x.co").await;
    app.post_form(
        "/projects",
        &[("name", "Other"), ("slug", "other")],
        Some(&budi),
    )
    .await;
    assert_eq!(
        app.get("/projects/other/monitors/1", Some(&budi))
            .await
            .status(),
        404
    );
}
