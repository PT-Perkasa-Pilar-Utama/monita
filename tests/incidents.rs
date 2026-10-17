//! US-15 to US-20. AC: docs/business/acceptance-criteria-breakdown/acceptance-criteria-sprint-2.md.

mod common;

use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU16, Ordering},
    },
    time::Duration,
};

use axum::{
    Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
use common::{RESET_TOKEN, TestApp, json_of};
use monita::{
    checks::{
        self, MonitorInput,
        heartbeat::{self, HeartbeatInput},
    },
    platform::db::query_i64,
};
use serde_json::{Value, json};

/// `/health` answers 200 or 503 depending on the switch.
async fn switchable() -> (String, Arc<AtomicBool>) {
    let up = Arc::new(AtomicBool::new(true));
    let app = Router::new()
        .route(
            "/health",
            get(|State(up): State<Arc<AtomicBool>>| async move {
                if up.load(Ordering::SeqCst) {
                    StatusCode::OK
                } else {
                    StatusCode::SERVICE_UNAVAILABLE
                }
            }),
        )
        .with_state(up.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (format!("http://{addr}/health"), up)
}

/// Webhook bodies the sink received.
type Received = Arc<Mutex<Vec<Value>>>;

/// `/slow` answers after 800 ms, so probes are still running when a test acts.
async fn slow_target() -> String {
    let app = Router::new().route(
        "/slow",
        get(|| async {
            tokio::time::sleep(Duration::from_millis(800)).await;
            "ok"
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    format!("http://{addr}/slow")
}

/// Records webhook bodies and answers with the status in the cell, after 300 ms.
async fn webhook_sink() -> (String, Received, Arc<AtomicU16>) {
    let got: Received = Arc::default();
    let status = Arc::new(AtomicU16::new(200));
    let app = Router::new()
        .route(
            "/hook",
            post(
                |State((got, status)): State<(Received, Arc<AtomicU16>)>, body: String| async move {
                    got.lock()
                        .unwrap()
                        .push(serde_json::from_str(&body).unwrap());
                    tokio::time::sleep(Duration::from_millis(300)).await;
                    StatusCode::from_u16(status.load(Ordering::SeqCst)).unwrap()
                },
            ),
        )
        .with_state((got.clone(), status.clone()));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (format!("http://{addr}/hook"), got, status)
}

/// Alert emails only: the TL's invite goes through the same mailer.
fn alerts(app: &TestApp) -> Vec<monita::notify::SentEmail> {
    app.state
        .mailer
        .sent()
        .into_iter()
        .filter(|m| !m.subject.contains("invited"))
        .collect()
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

/// Makes every monitor due and waits for one more check result.
async fn check_once(app: &TestApp) {
    let before = count(app, "SELECT count(*) FROM check_results").await;
    app.state
        .db
        .write(async |c| {
            c.execute("UPDATE monitors SET next_check_at = 0", ())
                .await?;
            Ok(())
        })
        .await
        .unwrap();
    assert!(
        wait_for(app, "SELECT count(*) FROM check_results", before + 1).await > before,
        "no check ran"
    );
}

async fn setup(app: &TestApp) -> String {
    let tl = app.tl("rina@x.co").await;
    app.project(&tl, "Payments", "payments").await;
    tl
}

async fn monitor(app: &TestApp, url: &str, severity: &str) {
    let m = MonitorInput {
        name: "API health".into(),
        url: url.into(),
        severity: severity.into(),
        ..MonitorInput::defaults()
    };
    checks::create_monitor(&app.state.db, 1, &m).await.unwrap();
}

#[tokio::test]
async fn the_resolved_alert_goes_where_the_opened_one_went() {
    let app = TestApp::spawn("test").await;
    let tl = setup(&app).await;
    for (severity, email) in [("warning", "warn@x.co"), ("critical", "crit@x.co")] {
        let res = app
            .api_send(
                reqwest::Method::PUT,
                &format!("/api/ui/projects/payments/alerts/{severity}"),
                serde_json::json!({ "emails": email, "webhook_url": "" }),
                Some(&tl),
            )
            .await;
        assert_eq!(res.status(), 204);
    }
    let (url, up) = switchable().await;
    monitor(&app, &url, "warning").await;
    up.store(false, Ordering::SeqCst);
    check_once(&app).await;
    check_once(&app).await;
    app.state
        .db
        .write(async |c| {
            c.execute("UPDATE monitors SET severity = 'critical'", ())
                .await?;
            Ok(())
        })
        .await
        .unwrap();
    up.store(true, Ordering::SeqCst);
    check_once(&app).await;
    monita::notify::deliver_due(&app.state).await.unwrap();
    let to: Vec<String> = alerts(&app).into_iter().map(|m| m.to).collect();
    assert_eq!(to, ["warn@x.co", "warn@x.co"]);
}

#[tokio::test]
async fn a_project_has_both_severities_with_the_default_routing() {
    // AC-15.01, AC-15.02
    let app = TestApp::spawn("test").await;
    let tl = setup(&app).await;
    let routes = app.get("/api/ui/projects/payments/alerts", Some(&tl)).await;
    let routes: serde_json::Value = routes.json().await.unwrap();
    let use_webhook: Vec<(&str, bool)> = routes["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            (
                r["severity"].as_str().unwrap(),
                r["use_webhook"].as_bool().unwrap(),
            )
        })
        .collect();
    assert_eq!(use_webhook, [("critical", true), ("warning", false)]);
    let bad = app
        .api_send(
            reqwest::Method::PUT,
            "/api/ui/projects/payments/alerts/critical",
            serde_json::json!({ "emails": "ok@x.co\nnot-an-email", "webhook_url": "" }),
            Some(&tl),
        )
        .await;
    assert_eq!(bad.status(), 422);
    let err: serde_json::Value = bad.json().await.unwrap();
    assert_eq!(
        err["error"]["fields"]["emails"],
        "Not a valid email address: not-an-email"
    );
}

#[tokio::test]
async fn two_failed_checks_open_one_incident_and_alert_then_a_pass_closes_it() {
    // AC-16.01, AC-16.02, AC-17.01, AC-18.01, AC-18.02
    let app = TestApp::spawn("test").await;
    let tl = setup(&app).await;
    let (hook, got, _) = webhook_sink().await;
    let res = app
        .api_send(
            reqwest::Method::PUT,
            "/api/ui/projects/payments/alerts/critical",
            serde_json::json!({ "emails": "oncall@x.co", "webhook_url": &hook, "use_webhook": true }),
            Some(&tl),
        )
        .await;
    assert_eq!(res.status(), 204);
    let (url, up) = switchable().await;
    monitor(&app, &url, "critical").await;

    up.store(false, Ordering::SeqCst);
    check_once(&app).await;
    assert_eq!(
        count(&app, "SELECT count(*) FROM incidents").await,
        0,
        "one failure is not an outage"
    );
    check_once(&app).await;
    check_once(&app).await;
    assert_eq!(
        count(
            &app,
            "SELECT count(*) FROM incidents WHERE resolved_at IS NULL"
        )
        .await,
        1,
        "one incident, not one per failure"
    );

    monita::notify::deliver_due(&app.state).await.unwrap();
    let mail = alerts(&app);
    assert_eq!(mail.len(), 1);
    assert_eq!(mail[0].to, "oncall@x.co");
    assert_eq!(mail[0].subject, "[CRITICAL] Payments / API health is down");
    assert!(
        mail[0]
            .body
            .contains("HTTP 503, expected 2xx after 2 checks")
    );
    assert!(
        mail[0]
            .body
            .contains("https://monita.test/projects/payments/incidents/1")
    );
    // The app's own outbox loop may have claimed the webhook row first; wait for it to land.
    for _ in 0..50 {
        if !got.lock().unwrap().is_empty() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    {
        let hooks = got.lock().unwrap();
        assert_eq!(hooks[0]["event"], "incident.opened");
        assert_eq!(hooks[0]["severity"], "critical");
        assert!(
            hooks[0]["text"]
                .as_str()
                .unwrap()
                .starts_with("[CRITICAL] Payments / API health is down")
        );
    }
    let list = json_of(
        app.api_get("/api/ui/projects/payments/monitors", Some(&tl))
            .await,
    )
    .await;
    assert_eq!(list["data"]["monitors"][0]["state"], "down");

    up.store(true, Ordering::SeqCst);
    check_once(&app).await;
    assert_eq!(
        count(
            &app,
            "SELECT count(*) FROM incidents WHERE resolved_at IS NOT NULL"
        )
        .await,
        1
    );
    monita::notify::deliver_due(&app.state).await.unwrap();
    let mail = alerts(&app);
    assert!(
        mail[1]
            .subject
            .starts_with("[RESOLVED] Payments / API health is up after")
    );
    assert_eq!(got.lock().unwrap()[1]["event"], "incident.resolved");

    let detail = incident(&app, &tl, 1).await;
    assert!(detail["incident"]["started_at"].is_i64());
    assert!(detail["incident"]["resolved_at"].is_i64(), "ended");
    let text = detail["updates"].to_string();
    for part in ["Recovered after", "Detected: HTTP 503"] {
        assert!(text.contains(part), "missing {part}");
    }
}

#[tokio::test]
async fn a_failure_between_passes_opens_nothing() {
    // AC-17.02
    let app = TestApp::spawn("test").await;
    setup(&app).await;
    let (url, up) = switchable().await;
    monitor(&app, &url, "warning").await;
    for down in [true, false, true, false] {
        up.store(!down, Ordering::SeqCst);
        check_once(&app).await;
    }
    assert_eq!(count(&app, "SELECT count(*) FROM incidents").await, 0);
}

#[tokio::test]
async fn warning_alerts_skip_the_webhook_unless_turned_on() {
    // AC-15.02
    let app = TestApp::spawn("test").await;
    let tl = setup(&app).await;
    let (hook, got, _) = webhook_sink().await;
    app.api_send(
        reqwest::Method::PUT,
        "/api/ui/projects/payments/alerts/warning",
        serde_json::json!({ "emails": "dev@x.co", "webhook_url": &hook }),
        Some(&tl),
    )
    .await;
    let (url, up) = switchable().await;
    monitor(&app, &url, "warning").await;
    up.store(false, Ordering::SeqCst);
    check_once(&app).await;
    check_once(&app).await;
    monita::notify::deliver_due(&app.state).await.unwrap();
    assert_eq!(alerts(&app).len(), 1);
    assert!(
        got.lock().unwrap().is_empty(),
        "webhook got {:?}",
        got.lock().unwrap()
    );
}

#[tokio::test]
async fn heartbeat_miss_and_fail_ping_open_at_once_and_a_ping_closes() {
    // AC-14.01, AC-17.03, AC-18.01
    let app = TestApp::spawn("test").await;
    let tl = setup(&app).await;
    heartbeat::create(
        &app.state.db,
        1,
        &HeartbeatInput {
            name: "nightly".into(),
            ..HeartbeatInput::default()
        },
    )
    .await
    .unwrap();
    let token = heartbeat::find(&app.state.db, 1, 1)
        .await
        .unwrap()
        .unwrap()
        .token;

    app.state
        .db
        .write(async |c| {
            c.execute("UPDATE heartbeats SET deadline_at = 0", ())
                .await?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        wait_for(
            &app,
            "SELECT count(*) FROM incidents WHERE resolved_at IS NULL",
            1
        )
        .await,
        1
    );
    let list = json_of(
        app.api_get("/api/ui/projects/payments/incidents", Some(&tl))
            .await,
    )
    .await;
    assert_eq!(
        list["data"]["incidents"][0]["title"],
        "nightly missed its heartbeat"
    );

    let ping = app
        .http
        .get(app.url(&format!("/ping/{token}")))
        .send()
        .await
        .unwrap();
    assert_eq!(ping.status(), 200);
    assert_eq!(
        count(
            &app,
            "SELECT count(*) FROM incidents WHERE resolved_at IS NULL"
        )
        .await,
        0
    );

    tokio::time::sleep(Duration::from_millis(1_100)).await;
    let fail = app
        .http
        .post(app.url(&format!("/ping/{token}/fail")))
        .body("backup.sh: disk full")
        .send()
        .await
        .unwrap();
    assert_eq!(fail.status(), 200);
    assert_eq!(
        count(
            &app,
            "SELECT count(*) FROM incidents WHERE resolved_at IS NULL"
        )
        .await,
        1
    );
    let detail = incident(&app, &tl, 2).await;
    assert_eq!(
        detail["incident"]["cause"], "backup.sh: disk full",
        "the fail body is the cause"
    );
}

#[tokio::test]
async fn status_updates_need_a_message_and_work_after_close() {
    // AC-19.01, AC-19.02, AC-19.03
    let app = TestApp::spawn("test").await;
    let tl = setup(&app).await;
    let (url, up) = switchable().await;
    monitor(&app, &url, "warning").await;
    up.store(false, Ordering::SeqCst);
    check_once(&app).await;
    check_once(&app).await;

    let empty = app
        .api_post(
            "/api/ui/projects/payments/incidents/1/updates",
            json!({ "label": "identified", "message": "  " }),
            Some(&tl),
        )
        .await;
    assert_eq!(empty.status(), 422);
    assert!(empty.text().await.unwrap().contains("Write a message."));
    let ok = app
        .api_post(
            "/api/ui/projects/payments/incidents/1/updates",
            json!({ "label": "identified", "message": "DB failover" }),
            Some(&tl),
        )
        .await;
    assert_eq!(ok.status(), 204);
    assert_eq!(
        incident(&app, &tl, 1).await["labels"],
        json!(["investigating", "identified", "monitoring"]),
        "an open automatic incident offers every label but Resolved"
    );
    let watching = app
        .api_post(
            "/api/ui/projects/payments/incidents/1/updates",
            json!({ "label": "monitoring", "message": "Watching the new node" }),
            Some(&tl),
        )
        .await;
    assert_eq!(watching.status(), 204);
    let note = app
        .api_post(
            "/api/ui/projects/payments/incidents/1/updates",
            json!({ "label": "resolved", "message": "Looks fixed" }),
            Some(&tl),
        )
        .await;
    assert_eq!(
        note.status(),
        422,
        "only a passing check resolves an automatic incident"
    );
    assert!(
        note.text()
            .await
            .unwrap()
            .contains("Monita resolves this incident when its check passes.")
    );

    up.store(true, Ordering::SeqCst);
    check_once(&app).await;
    let after = app
        .api_post(
            "/api/ui/projects/payments/incidents/1/updates",
            json!({ "label": "resolved", "message": "Root cause: a full disk" }),
            Some(&tl),
        )
        .await;
    assert_eq!(after.status(), 204);
    let reopened = app
        .api_post(
            "/api/ui/projects/payments/incidents/1/updates",
            json!({ "label": "investigating", "message": "Still looking" }),
            Some(&tl),
        )
        .await;
    assert_eq!(
        reopened.status(),
        422,
        "a closed incident takes only Resolved notes"
    );
    let detail = incident(&app, &tl, 1).await;
    let updates = detail["updates"].to_string();
    assert!(
        updates.contains("Root cause: a full disk")
            && updates.contains("DB failover")
            && updates.contains("rina@x.co")
    );
    assert_eq!(detail["labels"], json!(["resolved"]), "closed: notes only");
}

#[tokio::test]
async fn a_manual_incident_marks_its_items_and_closes_on_resolved() {
    // AC-20.01, AC-20.02
    let app = TestApp::spawn("test").await;
    let tl = setup(&app).await;
    monitor(&app, "https://x.invalid/", "warning").await;
    let choices = json_of(
        app.api_get("/api/ui/projects/payments/incident-items", Some(&tl))
            .await,
    )
    .await;
    assert_eq!(
        choices["data"],
        json!([{ "value": "monitor:1", "name": "API health" }])
    );
    let none = app
        .api_post(
"/api/ui/projects/payments/incidents",
json!({ "title": "Slow checkout", "items": [], "severity": "warning", "message": "Some users wait 20 s" }),
Some(&tl),
)
        .await;
    assert_eq!(none.status(), 422);
    assert!(
        none.text()
            .await
            .unwrap()
            .contains("Pick at least one monitor or heartbeat.")
    );
    let long = "x".repeat(10_001);
    let too_long = app
        .api_post(
"/api/ui/projects/payments/incidents",
json!({ "title": "Slow checkout", "items": ["monitor:1"], "severity": "warning", "message": long }),
Some(&tl),
)
        .await;
    assert_eq!(
        too_long.status(),
        422,
        "a message reaches the public page, so it is bounded"
    );
    let long_title = "t".repeat(121);
    let res = app
        .api_post(
"/api/ui/projects/payments/incidents",
json!({ "title": long_title, "items": ["monitor:1"], "severity": "warning", "message": "m" }),
Some(&tl),
)
        .await;
    assert!(
        res.text()
            .await
            .unwrap()
            .contains("The title needs at most 120 characters.")
    );
    let res = app
        .api_post(
"/api/ui/projects/payments/incidents",
json!({ "title": "Slow checkout", "items": ["monitor:1", "monitor:1"], "severity": "critical", "message": "Some users wait 20 s" }),
Some(&tl),
)
        .await;
    assert_eq!(res.status(), 201);
    assert_eq!(json_of(res).await["data"]["id"], 1);
    assert_eq!(
        count(&app, "SELECT count(*) FROM incident_items").await,
        1,
        "a repeated item is stored once"
    );
    let list = json_of(
        app.api_get("/api/ui/projects/payments/monitors", Some(&tl))
            .await,
    )
    .await;
    assert_eq!(list["data"]["monitors"][0]["state"], "issue");
    assert_eq!(
        incident(&app, &tl, 1).await["labels"],
        json!(["investigating", "identified", "monitoring", "resolved"]),
        "a reported incident offers all four labels"
    );

    app.api_post(
        "/api/ui/projects/payments/incidents/1/updates",
        json!({ "label": "resolved", "message": "CDN fixed" }),
        Some(&tl),
    )
    .await;
    assert_eq!(
        count(
            &app,
            "SELECT count(*) FROM incidents WHERE resolved_at IS NOT NULL"
        )
        .await,
        1
    );
}

#[tokio::test]
async fn a_failing_webhook_is_retried_later_with_the_error_kept() {
    let app = TestApp::spawn("test").await;
    let tl = setup(&app).await;
    let (hook, _, status) = webhook_sink().await;
    status.store(500, Ordering::SeqCst);
    app.api_send(
        reqwest::Method::PUT,
        "/api/ui/projects/payments/alerts/critical",
        serde_json::json!({ "webhook_url": &hook, "use_webhook": true }),
        Some(&tl),
    )
    .await;
    let res = app
        .api_post(
            "/api/ui/projects/payments/alerts/critical/test",
            serde_json::json!({}),
            Some(&tl),
        )
        .await;
    assert_eq!(res.status(), 200);
    let sent: serde_json::Value = res.json().await.unwrap();
    assert_eq!(sent["data"]["sent_to"], 1);
    monita::notify::deliver_due(&app.state).await.unwrap();
    assert_eq!(count(&app, "SELECT attempts FROM outbox").await, 1);
    let next = count(&app, "SELECT next_attempt_at FROM outbox").await;
    assert!(
        next > monita::platform::now() + 50,
        "retried in about a minute"
    );
    assert_eq!(
        count(
            &app,
            "SELECT count(*) FROM outbox WHERE last_error = 'webhook answered HTTP 500'"
        )
        .await,
        1
    );
}

#[tokio::test]
async fn deleting_a_monitor_closes_its_open_incident() {
    let app = TestApp::spawn("test").await;
    let tl = setup(&app).await;
    app.api_send(
        reqwest::Method::PUT,
        "/api/ui/projects/payments/alerts/warning",
        serde_json::json!({ "emails": "oncall@x.co", "webhook_url": "" }),
        Some(&tl),
    )
    .await;
    let (url, up) = switchable().await;
    monitor(&app, &url, "warning").await;
    up.store(false, Ordering::SeqCst);
    check_once(&app).await;
    check_once(&app).await;
    let res = app
        .api_send(
            reqwest::Method::DELETE,
            "/api/ui/projects/payments/monitors/1",
            serde_json::json!({}),
            Some(&tl),
        )
        .await;
    assert_eq!(res.status(), 204);
    assert_eq!(
        count(
            &app,
            "SELECT count(*) FROM incidents WHERE resolved_at IS NULL"
        )
        .await,
        0
    );
    monita::notify::deliver_due(&app.state).await.unwrap();
    let subjects: Vec<String> = alerts(&app).into_iter().map(|m| m.subject).collect();
    assert!(
        subjects
            .iter()
            .any(|s| s.starts_with("[RESOLVED] Payments / API health was deleted after")),
        "whoever got the opened alert hears the end: {subjects:?}"
    );
}

#[tokio::test]
async fn reset_state_waits_for_running_checks_so_no_stale_result_lands() {
    let app = TestApp::spawn("test").await;
    setup(&app).await;
    let url = slow_target().await;
    for _ in 0..3 {
        monitor(&app, &url, "warning").await;
    }
    app.state
        .db
        .write(async |c| {
            c.execute("UPDATE monitors SET next_check_at = 0", ())
                .await?;
            Ok(())
        })
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(1_200)).await; // probes are now in flight, ~400 ms left
    let reset = app
        .http
        .post(app.url("/admin/reset-state"))
        .header("x-reset-token", RESET_TOKEN)
        .json(&serde_json::json!({"seed": "qa"}));
    let res = tokio::time::timeout(Duration::from_secs(10), reset.send())
        .await
        .expect("reset hung")
        .unwrap();
    assert_eq!(res.status(), 204);
    // A monitor created right after the reset reuses id 1 (the QA seed's TL is user 2).
    // A probe from before the reset must not write to it.
    app.state
        .db
        .write(async |c| {
            c.execute_batch(
                "INSERT INTO projects (id, owner_id, name, slug) VALUES (1, 2, 'New', 'new');
                 INSERT INTO monitors (id, project_id, name, url, next_check_at) VALUES (1, 1, 'fresh', 'https://x.invalid/', 9999999999);",
            )
            .await?;
            Ok(())
        })
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(1_000)).await;
    assert_eq!(
        count(&app, "SELECT count(*) FROM check_results").await,
        0,
        "a stale result landed after the reset"
    );
}

async fn incident(app: &TestApp, tl: &str, id: i64) -> Value {
    let path = format!("/api/ui/projects/payments/incidents/{id}");
    json_of(app.api_get(&path, Some(tl)).await).await["data"].take()
}
