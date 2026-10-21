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

/// An alert as `incidents::observe` hands it to notify, for project 1.
fn opened(severity: &str) -> monita::incidents::Event {
    monita::incidents::Event::Opened {
        incident_id: 1,
        project_id: 1,
        source_name: "API".into(),
        title: "API is down".into(),
        severity: severity.into(),
        cause: "HTTP 503".into(),
    }
}

async fn send(app: &TestApp, ev: &monita::incidents::Event) {
    app.state
        .db
        .write(async |c| {
            monita::notify::send(c, ev, common::BASE_URL).await?;
            Ok(())
        })
        .await
        .unwrap();
}

/// Empties the outbox: setup queued invite emails.
async fn clear_outbox(app: &TestApp) {
    app.state
        .db
        .write(async |c| {
            c.execute("DELETE FROM outbox", ()).await?;
            Ok(())
        })
        .await
        .unwrap();
}

/// Email recipients in the outbox, sorted.
async fn mailed(app: &TestApp) -> Vec<String> {
    monita::platform::db::query_all(
        &app.state.db.read().await.unwrap(),
        "SELECT recipient FROM outbox WHERE channel = 'email' ORDER BY recipient",
        (),
        |r| r.get::<String>(0),
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn a_card_switched_off_sends_nothing_and_keeps_its_recipients() {
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    app.project(&tl, "Payments", "payments").await;
    let id = app
        .email_only(&tl, "payments", "critical", "oncall@x.co")
        .await;
    clear_outbox(&app).await;
    let switch = async |id: i64, on: bool| {
        app.api_send(
            reqwest::Method::PUT,
            &format!("/api/ui/projects/payments/alert-cards/{id}/enabled"),
            serde_json::json!({ "enabled": on }),
            Some(&tl),
        )
        .await
        .status()
    };
    assert_eq!(switch(id, false).await, 204);
    send(&app, &opened("critical")).await;
    assert!(mailed(&app).await.is_empty(), "off: no alert");
    let cards = app.alert_cards(&tl, "payments").await;
    let card = cards
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == id)
        .unwrap();
    assert_eq!(
        card["emails"],
        serde_json::json!(["oncall@x.co"]),
        "recipients stay"
    );
    assert_eq!(card["enabled"], false);

    assert_eq!(switch(id, true).await, 204);
    send(&app, &opened("critical")).await;
    assert_eq!(
        mailed(&app).await,
        ["oncall@x.co"],
        "back on: the alert goes out"
    );
    assert_eq!(switch(9999, false).await, 404);
}

/// v1.1 Q34: with no card listening, an alert goes nowhere and nothing fails: the incident that
/// caused it, the check and the ping go on as usual.
#[tokio::test]
async fn with_no_card_listening_an_alert_goes_nowhere_without_an_error() {
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    app.project(&tl, "Payments", "payments").await;
    app.owner_cards_off(&tl, "payments").await;
    clear_outbox(&app).await;
    send(&app, &opened("critical")).await;
    send(&app, &opened("warning")).await;
    let cert = monita::notify::CertWarning {
        project_id: 1,
        monitor_id: 1,
        monitor_name: "API",
        expires_at: monita::platform::now() + 86_400,
    };
    let queued = app
        .state
        .db
        .write(async |c| monita::notify::send_cert_warning(c, &cert, common::BASE_URL).await)
        .await
        .unwrap();
    assert!(!queued, "no warning card: nothing queued");
    let rows = query_i64(
        &app.state.db.read().await.unwrap(),
        "SELECT count(*) FROM outbox",
        (),
    )
    .await
    .unwrap();
    assert_eq!(rows, Some(0));

    let hb = monita::checks::heartbeat::create(
        &app.state.db,
        1,
        &monita::checks::heartbeat::HeartbeatInput {
            name: "nightly".into(),
            ..Default::default()
        },
    )
    .await
    .unwrap()
    .unwrap();
    let token = monita::checks::heartbeat::find(&app.state.db, 1, hb)
        .await
        .unwrap()
        .unwrap()
        .token;
    let fail = app
        .http
        .post(app.url(&format!("/ping/{token}/fail")))
        .body("disk full")
        .send()
        .await
        .unwrap();
    assert_eq!(fail.status(), 200, "the ping is heard");
    let open = query_i64(
        &app.state.db.read().await.unwrap(),
        "SELECT count(*) FROM incidents WHERE resolved_at IS NULL",
        (),
    )
    .await
    .unwrap();
    assert_eq!(open, Some(1), "the incident opens though no one is alerted");
}

/// An address in several cards that hear one alert gets one email.
#[tokio::test]
async fn an_address_in_two_cards_gets_one_email() {
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    app.project(&tl, "Payments", "payments").await;
    app.alert_card(
        &tl,
        "payments",
        serde_json::json!({ "kind": "email", "name": "Team", "critical": true, "warning": true,
                            "emails": "rina@x.co\noncall@x.co" }),
    )
    .await;
    clear_outbox(&app).await;
    send(&app, &opened("critical")).await;
    assert_eq!(mailed(&app).await, ["oncall@x.co", "rina@x.co"]);
}

/// AC-15.02: with the setup unchanged, both levels email the project's owner, also when a super
/// admin made the project for a tech lead.
#[tokio::test]
async fn a_new_project_emails_its_owner() {
    let app = TestApp::spawn("test").await;
    let admin = app.super_admin("admin@x.co").await;
    let tl = app.tl("rina@x.co").await;
    let made = app
        .api_post(
            "/api/ui/projects",
            serde_json::json!({ "name": "Payments", "homepage_url": "https://pay.example.com", "owner_email": "rina@x.co" }),
            Some(&admin),
        )
        .await;
    assert_eq!(made.status(), 201);
    let slug = common::json_of(made).await["data"]["slug"]
        .as_str()
        .unwrap()
        .to_string();
    clear_outbox(&app).await;
    let cards = app.alert_cards(&tl, &slug).await;
    let warning = cards
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["kind"] == "owner_email" && c["warning"] == true)
        .unwrap()["id"]
        .clone();
    let test = app
        .api_post(
            &format!("/api/ui/projects/{slug}/alert-cards/{warning}/test"),
            serde_json::json!({}),
            Some(&tl),
        )
        .await;
    assert_eq!(common::json_of(test).await["data"]["sent_to"], 1);
    assert_eq!(mailed(&app).await, ["rina@x.co"]);
}
