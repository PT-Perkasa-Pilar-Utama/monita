//! US-21, US-22. The public status page and its JSON.

mod common;

use common::TestApp;
use monita::{
    checks::{self, MonitorInput},
    platform::now,
};
use serde_json::Value;

const PAGE: &str = "/api/ui/public/payments/status";
const HISTORY: &str = "/api/ui/public/payments/history";

/// A public JSON response as text, for checks on what it must never carry.
async fn text(app: &TestApp, path: &str) -> String {
    let res = app.get(path, None).await;
    assert_eq!(res.status(), 200, "{path}");
    res.text().await.unwrap()
}

/// A project with one public and one hidden monitor, and an incident on the public one that
/// Monita detected (cause names an internal host) and the TL updated.
async fn setup(app: &TestApp) -> String {
    let tl = app.tl("rina@x.co").await;
    app.project(&tl, "Payments", "payments").await;
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
                 INSERT INTO incident_sources VALUES ('monitor', 1, 2, 1, NULL);
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
    let res = app.get(PAGE, None).await;
    assert_eq!(res.status(), 200);
    assert_eq!(res.headers()["cache-control"], "public, max-age=30");
    let page: Value = res.json().await.unwrap();
    let html = page.to_string();
    assert!(html.contains("Checkout is down"));
    assert!(html.contains("A DNS change rolled back."));
    let days = page["data"]["status"]["items"][0]["days"]
        .as_array()
        .unwrap();
    assert_eq!(days.len(), 90, "90 day bars");
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
    // The 600 s can straddle UTC midnight, so it is split across the last two bars.
    let downtime = |d: usize| item["days"][d]["downtime_s"].as_i64().unwrap();
    assert!(downtime(5) + downtime(6) >= 600);
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
    app.project(&tl, "New", "new").await;
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
    assert_eq!(
        app.get("/api/ui/public/nope/status", None).await.status(),
        404
    );
    let res = app.get("/api/v1/public/status/nope", None).await;
    assert_eq!(res.status(), 404);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["error"]["code"], "not_found");
}

#[tokio::test]
async fn a_bad_days_value_gets_the_error_envelope() {
    let app = TestApp::spawn("test").await;
    setup(&app).await;
    for days in ["abc", "0", "91"] {
        let res = app
            .get(&format!("/api/v1/public/status/payments?days={days}"), None)
            .await;
        assert_eq!(res.status(), 400, "days={days}");
        let body: Value = res.json().await.unwrap();
        assert_eq!(body["error"]["code"], "invalid_request");
    }
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
    let html = text(&app, PAGE).await;
    assert!(
        html.contains("A DNS change rolled back."),
        "the public incident is still listed"
    );
}

#[tokio::test]
async fn monitors_list_before_heartbeats_whatever_their_names() {
    let app = TestApp::spawn("test").await;
    setup(&app).await;
    let hb = monita::checks::heartbeat::HeartbeatInput {
        name: "Aardvark nightly".into(),
        is_public: true,
        ..Default::default()
    };
    monita::checks::heartbeat::create(&app.state.db, 1, &hb)
        .await
        .unwrap();
    let body: Value = app
        .get("/api/v1/public/status/payments", None)
        .await
        .json()
        .await
        .unwrap();
    let names: Vec<&str> = body["data"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["Checkout", "Aardvark nightly"]);
}

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

/// `n` rows from a recursive counter `n(i)`, i = 1..=n.
fn upto(n: i64) -> String {
    format!("WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < {n})")
}

#[tokio::test]
async fn a_status_page_with_200_systems_sends_each_with_its_90_days() {
    let app = TestApp::spawn("test").await;
    setup(&app).await;
    let t = now();
    let more = monita::platform::caps::SOURCES_PER_PROJECT - 2;
    sql(&app, &format!(
        "INSERT INTO monitors (project_id, name, url, is_public, next_check_at, created_at) {} SELECT 1, 'svc ' || i, 'https://x.invalid', 1, 9999999999, {t} FROM n;",
        upto(more)
    ))
    .await;
    let page: Value = app.get(PAGE, None).await.json().await.unwrap();
    let items = page["data"]["status"]["items"].as_array().unwrap();
    assert_eq!(items.len() as i64, more + 1, "the public one and the rest");
    assert!(
        items
            .iter()
            .all(|i| i["days"].as_array().unwrap().len() == 90)
    );
}

#[tokio::test]
async fn the_page_shows_open_incidents_and_the_newest_10_of_the_week_and_history_pages_the_rest() {
    let app = TestApp::spawn("test").await;
    setup(&app).await;
    let t = now();
    // 26 resolved incidents on the public monitor in the last days, and one open for 30 days.
    sql(&app, &format!(
        "UPDATE incidents SET resolved_at = {t} WHERE id = 1;
         INSERT INTO incidents (id, project_id, origin, title, severity, started_at, resolved_at) {} SELECT 100 + i, 1, 'auto', 'Blip ' || i, 'warning', {t} - i * 3600, {t} - i * 3600 + 60 FROM n;
         INSERT INTO incidents (id, project_id, origin, title, severity, started_at) VALUES (99, 1, 'auto', 'Long outage', 'critical', {t} - 30 * 86400);
         INSERT INTO incident_items SELECT id, 'monitor', 1 FROM incidents WHERE id >= 99;",
        upto(26)
    ))
    .await;
    let page = text(&app, PAGE).await;
    let json: Value = serde_json::from_str(&page).unwrap();
    let active = json["data"]["status"]["incidents"].to_string();
    assert!(
        active.contains("Long outage"),
        "an open incident shows however old"
    );
    let past = json["data"]["recent"].as_array().unwrap();
    assert_eq!(past.len() as i64, monita::platform::caps::PUBLIC_RECENT);
    assert!(
        !page.contains("Admin DB slow"),
        "an incident on a hidden system shows nowhere"
    );

    let first: Value = app.get(HISTORY, None).await.json().await.unwrap();
    let count = |v: &Value| v["data"]["incidents"].as_array().unwrap().len() as i64;
    assert_eq!(count(&first), monita::platform::caps::PUBLIC_HISTORY_PAGE);
    let before = first["data"]["older"].as_str().expect("an Older link");
    let second: Value = app
        .get(&format!("{HISTORY}?before={before}"), None)
        .await
        .json()
        .await
        .unwrap();
    // 26 blips, the resolved incident 1 and the long outage make 28: 25 then 3.
    assert_eq!(count(&second), 3);
    assert!(second["data"]["older"].is_null());
    let (first, second) = (first.to_string(), second.to_string());
    assert!(!first.contains("Admin DB slow") && !second.contains("Admin DB slow"));
    assert!(
        !first.contains("db-7.corp.internal"),
        "Monita's own cause stays private"
    );
}

#[tokio::test]
async fn past_incidents_older_than_a_week_leave_the_page_for_the_history() {
    let app = TestApp::spawn("test").await;
    setup(&app).await;
    let t = now();
    sql(&app, &format!(
        "INSERT INTO incidents (id, project_id, origin, title, severity, started_at, resolved_at) VALUES
           (50, 1, 'auto', 'Recent blip', 'warning', {r}, {r} + 60), (51, 1, 'auto', 'Old blip', 'warning', {o}, {o} + 60);
         INSERT INTO incident_items VALUES (50, 'monitor', 1), (51, 'monitor', 1);",
        r = t - 3 * 86_400,
        o = t - 20 * 86_400
    ))
    .await;
    let page = text(&app, PAGE).await;
    assert!(page.contains("Recent blip"));
    assert!(!page.contains("Old blip"));
    let history = text(&app, HISTORY).await;
    assert!(history.contains("Old blip"));
}

#[tokio::test]
async fn incidents_turned_off_leave_uptime_and_hide_every_incident() {
    let app = TestApp::spawn("test").await;
    let tl = setup(&app).await;
    let off = serde_json::json!({ "incidents": false, "known_issues": true, "report": true, "mine": true });
    let res = app
        .api_send(
            reqwest::Method::PUT,
            "/api/ui/projects/payments/public-pages",
            off,
            Some(&tl),
        )
        .await;
    assert_eq!(res.status(), 204);

    let page: Value = serde_json::from_str(&text(&app, PAGE).await).unwrap();
    assert_eq!(page["data"]["status"]["items"][0]["name"], "Checkout");
    assert_eq!(page["data"]["status"]["incidents"], serde_json::json!([]));
    assert_eq!(page["data"]["recent"], serde_json::json!([]));
    let json = text(&app, "/api/v1/public/status/payments").await;
    assert!(!json.contains("Checkout is down"), "{json}");
    assert_eq!(app.get(HISTORY, None).await.status(), 404);
}
