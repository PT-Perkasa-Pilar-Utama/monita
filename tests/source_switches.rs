//! Per-monitor switches: "Send alerts when it fails" and "Open an incident when it fails".

mod common;

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

use axum::{Router, extract::State, http::StatusCode, routing::get};
use common::{TestApp, json_of};
use monita::{
    checks::{self, MonitorInput},
    platform::db::query_i64,
};

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

async fn count(app: &TestApp, sql: &str) -> i64 {
    query_i64(&app.state.db.read().await.unwrap(), sql, ())
        .await
        .unwrap()
        .unwrap_or(0)
}

/// Makes every monitor due and waits for one more check result.
async fn check_once(app: &TestApp) {
    let sql = "SELECT count(*) FROM check_results";
    let before = count(app, sql).await;
    app.state
        .db
        .write(async |c| {
            c.execute("UPDATE monitors SET next_check_at = 0", ())
                .await?;
            Ok(())
        })
        .await
        .unwrap();
    for _ in 0..50 {
        if count(app, sql).await > before {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("no check ran");
}

/// Alert subjects sent so far; the invite email is left out.
async fn subjects(app: &TestApp) -> Vec<String> {
    monita::notify::deliver_due(&app.state).await.unwrap();
    app.state
        .mailer
        .sent()
        .into_iter()
        .filter(|m| m.to == "oncall@x.co")
        .map(|m| m.subject)
        .collect()
}

/// A tech lead, a project with oncall@x.co on critical alerts, and one public critical monitor.
async fn setup(app: &TestApp, alert: bool, auto_incident: bool) -> (String, Arc<AtomicBool>) {
    let tl = app.tl("rina@x.co").await;
    app.project(&tl, "Payments", "payments").await;
    let res = app
        .api_send(
            reqwest::Method::PUT,
            "/api/ui/projects/payments/alerts/critical",
            serde_json::json!({ "emails": "oncall@x.co", "webhook_url": "", "use_webhook": false }),
            Some(&tl),
        )
        .await;
    assert_eq!(res.status(), 204);
    let (url, up) = switchable().await;
    let m = MonitorInput {
        name: "API health".into(),
        url,
        severity: "critical".into(),
        is_public: true,
        alert,
        auto_incident,
        ..MonitorInput::defaults()
    };
    checks::create_monitor(&app.state.db, 1, &m).await.unwrap();
    (tl, up)
}

#[tokio::test]
async fn without_auto_incident_a_failure_alerts_down_and_up_and_opens_nothing() {
    let app = TestApp::spawn("test").await;
    let (tl, up) = setup(&app, true, false).await;

    up.store(false, Ordering::SeqCst);
    check_once(&app).await;
    check_once(&app).await;
    assert_eq!(count(&app, "SELECT count(*) FROM incidents").await, 0);
    assert_eq!(
        subjects(&app).await,
        ["[DOWN] Payments / API health is down"]
    );
    let team = json_of(
        app.api_get("/api/ui/projects/payments/monitors", Some(&tl))
            .await,
    )
    .await;
    assert_eq!(
        team["data"]["monitors"][0]["state"], "down",
        "the team sees it"
    );
    let items = json_of(
        app.api_get("/api/ui/projects/payments/incident-items", Some(&tl))
            .await,
    )
    .await;
    assert_eq!(
        (&items["data"][0]["kind"], &items["data"][0]["state"]),
        (&serde_json::json!("monitor"), &serde_json::json!("down")),
        "the incident form shows it is down"
    );
    let public = json_of(app.get("/api/ui/public/payments/status", None).await).await;
    assert!(
        !public.to_string().contains("\"down\""),
        "no incident, so the status page shows no outage"
    );

    check_once(&app).await;
    assert_eq!(subjects(&app).await.len(), 1, "one alert per outage");

    up.store(true, Ordering::SeqCst);
    check_once(&app).await;
    let sent = subjects(&app).await;
    assert_eq!(sent.len(), 2);
    assert!(
        sent[1].starts_with("[UP] Payments / API health is up after"),
        "{}",
        sent[1]
    );
    assert_eq!(count(&app, "SELECT count(*) FROM incidents").await, 0);
}

#[tokio::test]
async fn with_alerts_off_an_incident_opens_and_closes_without_a_message() {
    let app = TestApp::spawn("test").await;
    let (_tl, up) = setup(&app, false, true).await;

    up.store(false, Ordering::SeqCst);
    check_once(&app).await;
    check_once(&app).await;
    assert_eq!(
        count(
            &app,
            "SELECT count(*) FROM incidents WHERE resolved_at IS NULL"
        )
        .await,
        1
    );
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
    assert!(subjects(&app).await.is_empty());
    assert_eq!(
        count(&app, "SELECT count(*) FROM outbox").await,
        1,
        "the invite only"
    );
}

#[tokio::test]
async fn a_monitor_set_to_one_failure_opens_at_once_and_two_waits() {
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    app.project(&tl, "Payments", "payments").await;
    let (url, up) = switchable().await;
    for (name, fails_to_open) in [("at once", 1), ("waits", 2)] {
        let m = MonitorInput {
            name: name.into(),
            url: url.clone(),
            fails_to_open,
            ..MonitorInput::defaults()
        };
        checks::create_monitor(&app.state.db, 1, &m).await.unwrap();
    }
    up.store(false, Ordering::SeqCst);
    // Both monitors check in the same tick, so wait for two results.
    app.state
        .db
        .write(async |c| {
            c.execute("UPDATE monitors SET next_check_at = 0", ())
                .await?;
            Ok(())
        })
        .await
        .unwrap();
    for _ in 0..50 {
        if count(&app, "SELECT count(*) FROM check_results").await >= 2 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let open = "SELECT count(*) FROM incidents WHERE resolved_at IS NULL";
    assert_eq!(count(&app, open).await, 1, "only the one set to 1 opened");
    let title = "SELECT count(*) FROM incidents WHERE title = 'at once is down'";
    assert_eq!(count(&app, title).await, 1);

    let bad = serde_json::json!({ "name": "x", "url": url, "interval_s": "60", "timeout_s": "5",
        "expected_status": "2xx", "severity": "warning", "fails_to_open": "4" });
    let res = app
        .api_post("/api/ui/projects/payments/monitors", bad, Some(&tl))
        .await;
    assert_eq!(res.status(), 422);
    assert!(json_of(res).await["error"]["fields"]["fails_to_open"].is_string());
}
