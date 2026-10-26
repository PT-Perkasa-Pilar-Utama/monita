//! v1.1 Q46: an incident reclassified as planned work or a false alarm leaves uptime, the
//! charts and the public page; the team still sees it; restore counts it again.

mod common;

use common::{TestApp, json_of};
use monita::{
    checks::{self, MonitorInput},
    platform::now,
};
use serde_json::{Value, json};

/// One public monitor with a closed 10-minute automatic incident, and one with an open one.
async fn setup(app: &TestApp) -> String {
    let tl = app.tl("rina@x.co").await;
    app.project(&tl, "Payments", "payments").await;
    for name in ["Checkout", "Search"] {
        let m = MonitorInput {
            name: name.into(),
            url: "https://x.invalid/".into(),
            is_public: true,
            ..MonitorInput::defaults()
        };
        checks::create_monitor(&app.state.db, 1, &m).await.unwrap();
    }
    let t = now();
    app.state
        .db
        .write(async |c| {
            c.execute_batch(&format!(
                "UPDATE monitors SET created_at = {old}, next_check_at = 9999999999;
                 INSERT INTO check_results (monitor_id, checked_at, ok) VALUES (1, {t}, 1), (2, {t}, 0);
                 INSERT INTO incidents (id, project_id, origin, title, severity, started_at, resolved_at) VALUES (1, 1, 'auto', 'Checkout is down', 'critical', {start}, {end});
                 INSERT INTO incident_items VALUES (1, 'monitor', 1);
                 INSERT INTO status_updates (incident_id, label, message, created_at) VALUES (1, 'investigating', 'Detected: 503.', {start});
                 INSERT INTO incidents (id, project_id, origin, title, severity, started_at) VALUES (2, 1, 'auto', 'Search is down', 'critical', {start});
                 INSERT INTO incident_items VALUES (2, 'monitor', 2);
                 INSERT INTO incident_sources VALUES ('monitor', 2, 2, 2, NULL);",
                old = t - 200 * 86_400,
                start = t - 1200,
                end = t - 600,
            ))
            .await?;
            Ok(())
        })
        .await
        .unwrap();
    tl
}

async fn class(app: &TestApp, tl: &str, id: i64, class: &str) -> reqwest::StatusCode {
    app.api_send(
        reqwest::Method::PUT,
        &format!("/api/ui/projects/payments/incidents/{id}/class"),
        json!({ "class": class }),
        Some(tl),
    )
    .await
    .status()
}

async fn public(app: &TestApp) -> Value {
    json_of(app.get("/api/v1/public/status/payments?days=1", None).await).await["data"].clone()
}

fn downtime(data: &Value, name: &str) -> i64 {
    let item = data["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["name"] == name)
        .unwrap();
    item["days"][0]["downtime_s"].as_i64().unwrap()
}

#[tokio::test]
async fn planned_work_leaves_the_public_page_and_uptime_and_restore_brings_it_back() {
    let app = TestApp::spawn("test").await;
    let tl = setup(&app).await;
    let before = public(&app).await;
    assert!(downtime(&before, "Checkout") >= 600, "{before}");
    assert!(before.to_string().contains("Checkout is down"));

    assert_eq!(class(&app, &tl, 1, "planned").await, 204);
    let after = public(&app).await;
    assert_eq!(downtime(&after, "Checkout"), 0, "{after}");
    assert!(!after.to_string().contains("Checkout is down"));
    let team = json_of(
        app.get("/api/ui/projects/payments/incidents/1", Some(&tl))
            .await,
    )
    .await;
    assert_eq!(team["data"]["incident"]["excluded"], "planned");
    assert_eq!(team["data"]["incident"]["excluded_by"], "rina@x.co");

    assert_eq!(class(&app, &tl, 1, "false_alarm").await, 204);
    let team = json_of(
        app.get("/api/ui/projects/payments/incidents/1", Some(&tl))
            .await,
    )
    .await;
    assert_eq!(team["data"]["incident"]["excluded"], "false_alarm");

    assert_eq!(class(&app, &tl, 1, "counted").await, 204);
    let restored = public(&app).await;
    assert!(downtime(&restored, "Checkout") >= 600, "{restored}");
    let team = json_of(
        app.get("/api/ui/projects/payments/incidents/1", Some(&tl))
            .await,
    )
    .await;
    assert!(team["data"]["incident"]["excluded"].is_null());
    assert!(
        team["data"]["incident"]["resolved_at"].is_number(),
        "restore never reopens"
    );
}

#[tokio::test]
async fn reclassifying_an_open_incident_closes_it_with_a_note() {
    let app = TestApp::spawn("test").await;
    let tl = setup(&app).await;
    assert_eq!(class(&app, &tl, 2, "false_alarm").await, 204);
    let team = json_of(
        app.get("/api/ui/projects/payments/incidents/2", Some(&tl))
            .await,
    )
    .await;
    let incident = &team["data"]["incident"];
    assert!(incident["resolved_at"].is_number(), "{team}");
    assert_eq!(incident["excluded"], "false_alarm");
    let updates = team["data"]["updates"].as_array().unwrap();
    assert!(
        updates
            .iter()
            .any(|u| u["label"] == "resolved" && u["author_email"] == "rina@x.co"),
        "{team}"
    );
    assert_eq!(downtime(&public(&app).await, "Search"), 0);
}

#[tokio::test]
async fn a_bad_class_or_another_projects_incident_is_refused() {
    let app = TestApp::spawn("test").await;
    let tl = setup(&app).await;
    assert_eq!(class(&app, &tl, 1, "ignored").await, 422);
    assert_eq!(class(&app, &tl, 99, "planned").await, 404);
}
