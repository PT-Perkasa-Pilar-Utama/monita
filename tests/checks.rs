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
use common::{TestApp, json_of};
use monita::{
    checks::{self, Clients, MonitorInput, probe},
    platform::db::query_i64,
};
use reqwest::Method;
use serde_json::json;

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
    app.project(&tl, "Payments", "payments").await;
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
                method: "GET",
                headers: Vec::new(),
                body: None,
                timeout_ms,
                expected_status: "2xx",
                keyword,
                skip_tls_verify: false,
                auth: None,
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
        method: "GET",
        headers: Vec::new(),
        body: None,
        timeout_ms: 2000,
        expected_status,
        keyword: None,
        skip_tls_verify: false,
        auth: None,
    };
    let seen = probe(&check("301"), &clients).await;
    assert!(seen.ok, "{:?}", seen.error);
    assert_eq!(seen.status_code, Some(301));
    let followed = probe(&check("2xx"), &clients).await;
    assert!(followed.ok);
    assert_eq!(followed.status_code, Some(200));
}

#[tokio::test]
async fn check_options_offer_the_intervals_and_the_monitor_defaults() {
    // AC-09.01, AC-09.02
    let app = TestApp::spawn("test").await;
    let tl = tl_project(&app).await;
    let o = json_of(app.api_get("/api/ui/check-options", Some(&tl)).await).await;
    let intervals: Vec<i64> = o["data"]["intervals"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["value"].as_i64().unwrap())
        .collect();
    assert_eq!(intervals, [30, 60, 180, 300]);
    assert_eq!(
        o["data"]["monitor"],
        json!({ "interval_s": 60, "timeout_s": 10, "expected_status": "2xx", "severity": "warning" })
    );
}

#[tokio::test]
async fn tl_creates_a_monitor_and_bad_input_is_refused() {
    let app = TestApp::spawn("test").await;
    let tl = tl_project(&app).await;
    let form = json!({
        "name": "API health",
        "url": "https://pay.internal/health",
        "interval_s": "30",
        "timeout_s": "5",
        "expected_status": "200,204",
        "keyword": "",
        "severity": "critical",
        "is_public": true,
    });
    let res = app
        .api_post("/api/ui/projects/payments/monitors", form, Some(&tl))
        .await;
    assert_eq!(res.status(), 201);
    let id = json_of(res).await["data"]["id"].as_i64().unwrap();
    let detail = json_of(
        app.api_get(
            &format!("/api/ui/projects/payments/monitors/{id}"),
            Some(&tl),
        )
        .await,
    )
    .await;
    let m = &detail["data"]["monitor"];
    assert_eq!(m["name"], "API health");
    assert_eq!(m["expected_status"], "200,204");
    assert_eq!(m["severity"], "critical");
    assert_eq!(m["timeout_s"], 5);
    assert_eq!(m["is_public"], true);

    let bad = json!({
        "name": "",
        "url": "ftp://x",
        "interval_s": "45",
        "timeout_s": "99",
        "expected_status": "ok",
        "severity": "warning",
    });
    let res = app
        .api_post("/api/ui/projects/payments/monitors", bad, Some(&tl))
        .await;
    assert_eq!(res.status(), 422);
    let fields = &json_of(res).await["error"]["fields"];
    assert_eq!(
        fields["url"],
        "The URL must start with http:// or https://."
    );
    assert_eq!(
        fields["timeout_s"], "Use 1 to 30 seconds.",
        "the form's own name"
    );
    for f in ["name", "interval_s", "expected_status"] {
        assert!(fields[f].is_string(), "missing {f}");
    }
    assert_eq!(count(&app, "SELECT count(*) FROM monitors").await, 1);
}

#[tokio::test]
async fn tl_edits_and_deletes_a_monitor() {
    let app = TestApp::spawn("test").await;
    let tl = tl_project(&app).await;
    let mut form = json!({
        "name": "API", "url": "https://pay.internal/", "interval_s": "60",
        "timeout_s": "10", "expected_status": "2xx", "severity": "warning",
    });
    let res = app
        .api_post(
            "/api/ui/projects/payments/monitors",
            form.clone(),
            Some(&tl),
        )
        .await;
    let path = format!(
        "/api/ui/projects/payments/monitors/{}",
        json_of(res).await["data"]["id"]
    );
    form["name"] = json!("API v2");
    let res = app
        .api_send(Method::PUT, &path, form.clone(), Some(&tl))
        .await;
    assert_eq!(res.status(), 204);
    let detail = json_of(app.api_get(&path, Some(&tl)).await).await;
    assert_eq!(detail["data"]["monitor"]["name"], "API v2");

    let res = app
        .api_send(Method::DELETE, &path, json!({}), Some(&tl))
        .await;
    assert_eq!(res.status(), 204);
    assert_eq!(app.api_get(&path, Some(&tl)).await.status(), 404);
    let res = app.api_send(Method::PUT, &path, form, Some(&tl)).await;
    assert_eq!(res.status(), 404, "a deleted monitor cannot be saved");
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

    let detail = json_of(
        app.api_get("/api/ui/projects/payments/monitors/2", Some(&tl))
            .await,
    )
    .await;
    let check = &detail["data"]["checks"][0];
    assert_eq!(check["status_code"], 503);
    assert_eq!(check["ok"], false);
    assert_eq!(check["error"], "HTTP 503, expected 2xx");
    assert!(check["latency_ms"].is_i64(), "latency column");
    let list = json_of(
        app.api_get("/api/ui/projects/payments/monitors", Some(&tl))
            .await,
    )
    .await;
    let rows = list["data"]["monitors"].as_array().unwrap();
    assert!(
        rows.iter().all(|m| m["state"] != "down"),
        "AC-17.02: one failed check is not an outage"
    );
    let ok = rows.iter().find(|m| m["name"] == "OK page").unwrap();
    assert!(
        ok["last"]["summary"]
            .as_str()
            .unwrap()
            .starts_with("200 in ")
    );
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

/// Every runtime limit set to `n`, as the System page's Save sends it.
fn limits(n: i64) -> serde_json::Value {
    monita::platform::settings::KEYS
        .iter()
        .map(|(k, _)| (k.to_string(), json!(n)))
        .collect::<serde_json::Map<_, _>>()
        .into()
}

#[tokio::test]
async fn no_more_checks_run_at_once_than_the_cap() {
    // AC-11.02
    let app = TestApp::spawn("test").await;
    let admin = app.super_admin("admin@x.co").await;
    let mut form = limits(50);
    form["check_concurrency"] = json!(2);
    let res = app
        .api_send(Method::PUT, "/api/ui/admin/limits", form, Some(&admin))
        .await;
    assert_eq!(res.status(), 204);
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
    assert_eq!(
        app.get("/api/ui/admin/system", Some(&tl)).await.status(),
        403
    );
    let system = json_of(app.get("/api/ui/admin/system", Some(&admin)).await).await;
    let concurrency = system["data"]["limits"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["key"] == "check_concurrency")
        .unwrap();
    assert_eq!(concurrency["value"], 50);
    let mut bad = limits(50);
    bad["check_concurrency"] = json!(0);
    let res = app
        .api_send(Method::PUT, "/api/ui/admin/limits", bad, Some(&admin))
        .await;
    assert_eq!(res.status(), 422);
    let fields = &json_of(res).await["error"]["fields"];
    assert!(fields["check_concurrency"].is_string() && fields.as_object().unwrap().len() == 1);
    let mut huge = limits(50);
    huge["rl_key_per_min"] = json!("4294967296");
    let res = app
        .api_send(Method::PUT, "/api/ui/admin/limits", huge, Some(&admin))
        .await;
    assert_eq!(res.status(), 422, "a value that would wrap a u32");
    let set = app
        .state
        .db
        .write(async |c| monita::platform::settings::set(c, "check_concurrency", 501).await)
        .await;
    assert!(set.is_err(), "check_concurrency stops at 500");
}

#[tokio::test]
async fn a_blank_number_comes_back_as_that_fields_error() {
    let app = TestApp::spawn("test").await;
    let tl = tl_project(&app).await;
    let res = app
        .api_post(
            "/api/ui/projects/payments/monitors",
            json!({
                "name": "API", "url": "https://x.invalid/", "interval_s": "60",
                "timeout_s": "", "expected_status": "2xx", "severity": "warning",
            }),
            Some(&tl),
        )
        .await;
    assert_eq!(res.status(), 422);
    let fields = json_of(res).await["error"]["fields"].take();
    assert_eq!(fields, json!({ "timeout_s": "Use 1 to 30 seconds." }));
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
    let page = |q: String| {
        let (app, tl) = (&app, &tl);
        async move {
            let path = format!("/api/ui/projects/payments/monitors/1{q}");
            json_of(app.api_get(&path, Some(tl)).await).await["data"].take()
        }
    };
    let rows = |d: &serde_json::Value| d["checks"].as_array().unwrap().len();
    let first = page(String::new()).await;
    assert_eq!(rows(&first), 25);
    let before = first["older"].as_str().expect("an older-checks link");
    let second = page(format!("?before={before}")).await;
    assert_eq!(rows(&second), 1, "the 26th check is not skipped");
    assert!(second["older"].is_null());
}

#[tokio::test]
async fn another_project_cannot_open_a_monitor_by_id() {
    let app = TestApp::spawn("test").await;
    tl_project(&app).await;
    checks::create_monitor(&app.state.db, 1, &input("Secret", "https://x.invalid/"))
        .await
        .unwrap();
    let budi = app.tl("budi@x.co").await;
    app.project(&budi, "Other", "other").await;
    assert_eq!(
        app.api_get("/api/ui/projects/other/monitors/1", Some(&budi))
            .await
            .status(),
        404
    );
    let res = app
        .api_send(
            Method::DELETE,
            "/api/ui/projects/other/monitors/1",
            json!({}),
            Some(&budi),
        )
        .await;
    assert_eq!(res.status(), 204, "a delete of nothing is a no-op");
    assert_eq!(count(&app, "SELECT count(*) FROM monitors").await, 1);
}
