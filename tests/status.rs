//! US-21, US-22. The public status page and its JSON.

mod common;

use common::TestApp;
use monita::{
    checks::{self, MonitorInput},
    platform::now,
};
use serde_json::Value;

/// A project with one public and one hidden monitor, and an incident on the public one that
/// Monita detected (cause names an internal host) and the TL updated.
async fn setup(app: &TestApp) -> String {
    let tl = app.tl("rina@x.co").await;
    app.post_form(
        "/projects",
        &[("name", "Payments"), ("slug", "payments")],
        Some(&tl),
    )
    .await;
    let public = MonitorInput {
        name: "Checkout".into(),
        url: "https://x.invalid/".into(),
        is_public: true,
        ..MonitorInput::defaults()
    };
    let hidden = MonitorInput {
        name: "Internal DB admin".into(),
        url: "https://y.invalid/".into(),
        ..MonitorInput::defaults()
    };
    checks::create_monitor(&app.state.db, 1, &public)
        .await
        .unwrap();
    checks::create_monitor(&app.state.db, 1, &hidden)
        .await
        .unwrap();
    let t = now();
    app.state
        .db
        .write(async |c| {
            c.execute_batch(&format!(
                "UPDATE monitors SET created_at = {old}, next_check_at = 9999999999;
                 INSERT INTO check_results (monitor_id, checked_at, ok) VALUES (1, {t}, 0), (2, {t}, 1);
                 INSERT INTO incidents (id, project_id, origin, title, severity, cause, started_at) VALUES (1, 1, 'auto', 'Checkout is down', 'critical', 'dns error: db-7.corp.internal', {start});
                 INSERT INTO incident_items VALUES (1, 'monitor', 1);
                 INSERT INTO incident_sources VALUES ('monitor', 1, 2, 1);
                 INSERT INTO status_updates (incident_id, label, message, author_id, created_at) VALUES
                   (1, 'investigating', 'Detected: dns error: db-7.corp.internal.', NULL, {start}),
                   (1, 'identified', 'A DNS change rolled back.', 1, {t});
                 INSERT INTO incidents (id, project_id, origin, title, severity, started_at) VALUES (2, 1, 'manual', 'Admin DB slow', 'warning', {t});
                 INSERT INTO incident_items VALUES (2, 'monitor', 2);",
                old = t - 200 * 86_400,
                start = t - 600,
            ))
            .await?;
            Ok(())
        })
        .await
        .unwrap();
    tl
}

#[tokio::test]
async fn status_page_shows_public_items_their_bars_and_incidents() {
    // AC-21.01, AC-21.02, AC-06.03
    let app = TestApp::spawn("test").await;
    setup(&app).await;
    let res = app.get("/status/payments", None).await;
    assert_eq!(res.status(), 200);
    assert_eq!(res.headers()["cache-control"], "public, max-age=30");
    let html = res.text().await.unwrap();
    assert!(html.contains("Checkout is down"));
    assert!(html.contains("A DNS change rolled back."));
    assert_eq!(html.matches("title=\"20").count(), 90, "90 day bars");
    assert!(
        !html.contains("Internal DB admin"),
        "hidden monitors never appear"
    );
    assert!(
        !html.contains("Admin DB slow"),
        "incidents on hidden items never appear"
    );
    assert!(
        !html.contains("corp.internal"),
        "Monita's own cause text stays private"
    );
    assert!(!html.contains("rina@x.co"), "no author on the public page");
}

#[tokio::test]
async fn public_json_matches_the_page_and_allows_any_origin() {
    // AC-22.01
    let app = TestApp::spawn("test").await;
    setup(&app).await;
    let res = app.get("/api/v1/public/status/payments?days=7", None).await;
    assert_eq!(res.status(), 200);
    assert_eq!(res.headers()["access-control-allow-origin"], "*");
    let body: Value = res.json().await.unwrap();
    let data = &body["data"];
    assert_eq!(data["project"]["slug"], "payments");
    assert_eq!(data["overall"], "down");
    assert_eq!(data["items"].as_array().unwrap().len(), 1);
    let item = &data["items"][0];
    assert_eq!(item["name"], "Checkout");
    assert_eq!(item["state"], "down");
    assert_eq!(item["days"].as_array().unwrap().len(), 7);
    assert!(item["days"][6]["downtime_s"].as_i64().unwrap() >= 600);
    let incident = &data["incidents"][0];
    assert_eq!(incident["items"][0], "Checkout");
    assert_eq!(
        incident["updates"][0]["message"],
        "A DNS change rolled back."
    );
    assert_eq!(incident["updates"][1]["message"], "Detected.");
    assert!(incident["updates"][0].get("author").is_none());
    assert_eq!(data["incidents"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn days_before_an_item_existed_are_null() {
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    app.post_form("/projects", &[("name", "New"), ("slug", "new")], Some(&tl))
        .await;
    let m = MonitorInput {
        name: "API".into(),
        url: "https://x.invalid/".into(),
        is_public: true,
        ..MonitorInput::defaults()
    };
    checks::create_monitor(&app.state.db, 1, &m).await.unwrap();
    let body: Value = app
        .get("/api/v1/public/status/new?days=3", None)
        .await
        .json()
        .await
        .unwrap();
    let days = body["data"]["items"][0]["days"].as_array().unwrap().clone();
    assert!(days[0]["uptime"].is_null());
    assert!(days[1]["uptime"].is_null());
    assert_eq!(days[2]["uptime"], 100.0);
    assert_eq!(body["data"]["items"][0]["state"], "unknown", "no check yet");
    assert_eq!(body["data"]["overall"], "up");
}

#[tokio::test]
async fn an_unknown_slug_is_404() {
    let app = TestApp::spawn("test").await;
    assert_eq!(app.get("/status/nope", None).await.status(), 404);
    let res = app.get("/api/v1/public/status/nope", None).await;
    assert_eq!(res.status(), 404);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["error"]["code"], "not_found");
}

#[tokio::test]
async fn hidden_incidents_do_not_push_public_ones_off_the_page() {
    let app = TestApp::spawn("test").await;
    setup(&app).await;
    let t = now();
    app.state
        .db
        .write(async |c| {
            for i in 0..200 {
                c.execute(
                    "INSERT INTO incidents (project_id, origin, title, severity, started_at) VALUES (1, 'manual', 'Hidden', 'warning', ?1)",
                    [t - i],
                )
                .await?;
                let id = c.last_insert_rowid();
                c.execute("INSERT INTO incident_items VALUES (?1, 'monitor', 2)", [id])
                    .await?;
            }
            Ok(())
        })
        .await
        .unwrap();
    let html = app
        .get("/status/payments", None)
        .await
        .text()
        .await
        .unwrap();
    assert!(
        html.contains("A DNS change rolled back."),
        "the public incident is still listed"
    );
}
