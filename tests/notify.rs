//! Outbox delivery edge cases: redirects, secrets in errors.

mod common;

use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use axum::{
    Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
use common::TestApp;
use monita::platform::db::query_i64;

async fn queue_webhook(app: &TestApp, url: &str) {
    let url = url.to_string();
    app.state
        .db
        .write(async |c| {
            c.execute(
                "INSERT INTO outbox (channel, recipient, body, next_attempt_at) VALUES ('webhook', ?1, '{}', 0)",
                [url.as_str()],
            )
            .await?;
            Ok(())
        })
        .await
        .unwrap();
}

/// A webhook endpoint that answers after 300 ms and counts its calls, either as they arrive or as
/// they finish.
async fn counting_sink(count_on_arrival: bool) -> (std::net::SocketAddr, Arc<AtomicUsize>) {
    let hits = Arc::new(AtomicUsize::new(0));
    let sink = Router::new()
        .route(
            "/hook",
            post(move |State(h): State<Arc<AtomicUsize>>| async move {
                if count_on_arrival {
                    h.fetch_add(1, Ordering::SeqCst);
                }
                tokio::time::sleep(Duration::from_millis(300)).await;
                if !count_on_arrival {
                    h.fetch_add(1, Ordering::SeqCst);
                }
            }),
        )
        .with_state(hits.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, sink).await.unwrap() });
    (addr, hits)
}

async fn last_error(app: &TestApp) -> String {
    let conn = app.state.db.read().await.unwrap();
    let mut rows = conn
        .query("SELECT coalesce(last_error, '') FROM outbox", ())
        .await
        .unwrap();
    rows.next().await.unwrap().unwrap().get(0).unwrap()
}

#[tokio::test]
async fn a_webhook_that_redirects_is_a_failed_send() {
    let app = Router::new()
        .route(
            "/hook",
            post(|| async { (StatusCode::MOVED_PERMANENTLY, [("location", "/landing")]) }),
        )
        .route("/landing", get(|| async { "a page, not the hook" }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let monita = TestApp::spawn("test").await;
    queue_webhook(&monita, &format!("http://{addr}/hook")).await;
    monita::notify::deliver_due(&monita.state).await.unwrap();
    assert_eq!(last_error(&monita).await, "webhook answered HTTP 301");
    let sent = query_i64(
        &monita.state.db.read().await.unwrap(),
        "SELECT count(*) FROM outbox WHERE sent_at IS NOT NULL",
        (),
    )
    .await
    .unwrap();
    assert_eq!(sent, Some(0));
}

#[tokio::test]
async fn a_failed_webhook_never_stores_its_url() {
    let app = TestApp::spawn("test").await;
    queue_webhook(&app, "http://127.0.0.1:9/services/T0/B0/secret-token").await;
    monita::notify::deliver_due(&app.state).await.unwrap();
    let error = last_error(&app).await;
    assert!(!error.is_empty());
    assert!(!error.contains("secret-token"), "{error}");
}

#[tokio::test]
async fn a_delivery_pass_sends_five_messages_at_once() {
    let (addr, hits) = counting_sink(false).await;
    let app = TestApp::spawn("test").await;
    let started = std::time::Instant::now();
    for _ in 0..5 {
        queue_webhook(&app, &format!("http://{addr}/hook")).await;
    }
    monita::notify::deliver_due(&app.state).await.unwrap();
    while hits.load(Ordering::SeqCst) < 5 {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    // The sink answers after 300 ms: one at a time would take 1.5 s.
    assert!(
        started.elapsed() < Duration::from_millis(1_000),
        "{:?}",
        started.elapsed()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn two_delivery_passes_never_send_one_message_twice() {
    let (addr, hits) = counting_sink(true).await;
    let app = TestApp::spawn("test").await;
    queue_webhook(&app, &format!("http://{addr}/hook")).await;
    // The sink answers after 300 ms, so the second pass starts while the first is still sending.
    let first = tokio::spawn({
        let s = app.state.clone();
        async move { monita::notify::deliver_due(&s).await }
    });
    tokio::time::sleep(Duration::from_millis(100)).await;
    monita::notify::deliver_due(&app.state).await.unwrap();
    first.await.unwrap().unwrap();
    tokio::time::sleep(Duration::from_millis(1_500)).await;
    assert_eq!(hits.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn an_alert_switched_off_sends_nothing_and_keeps_its_recipients() {
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    app.project(&tl, "Payments", "payments").await;
    let route = "/api/ui/projects/payments/alerts/critical";
    let body =
        serde_json::json!({"emails": "oncall@x.co", "webhook_url": "", "use_webhook": false});
    app.api_send(reqwest::Method::PUT, route, body, Some(&tl))
        .await;
    let switch_path = format!("{route}/enabled");
    let switch = |on: bool| {
        app.api_send(
            reqwest::Method::PUT,
            &switch_path,
            serde_json::json!({ "enabled": on }),
            Some(&tl),
        )
    };
    assert_eq!(switch(false).await.status(), 204);

    let opened = monita::incidents::Event::Opened {
        incident_id: 1,
        project_id: 1,
        source_name: "API".into(),
        title: "API is down".into(),
        severity: "critical".into(),
        cause: "HTTP 503".into(),
    };
    let send = async || {
        app.state
            .db
            .write(async |c| {
                monita::notify::send(c, &opened, common::BASE_URL).await?;
                Ok(())
            })
            .await
            .unwrap();
    };
    send().await;
    let outbox = async || {
        query_i64(
            &app.state.db.read().await.unwrap(),
            "SELECT count(*) FROM outbox WHERE recipient = 'oncall@x.co'",
            (),
        )
        .await
        .unwrap()
        .unwrap_or(0)
    };
    assert_eq!(outbox().await, 0, "off: no alert");
    let routes = common::json_of(
        app.api_get("/api/ui/projects/payments/alerts", Some(&tl))
            .await,
    )
    .await;
    assert_eq!(
        routes["data"][0]["emails"],
        serde_json::json!(["oncall@x.co"]),
        "recipients stay"
    );
    assert_eq!(routes["data"][0]["enabled"], false);

    assert_eq!(switch(true).await.status(), 204);
    send().await;
    assert_eq!(outbox().await, 1, "back on: the alert goes out");

    let bad = app
        .api_send(
            reqwest::Method::PUT,
            "/api/ui/projects/payments/alerts/info/enabled",
            serde_json::json!({ "enabled": false }),
            Some(&tl),
        )
        .await;
    assert_eq!(bad.status(), 404);
}
