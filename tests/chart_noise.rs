//! Paused, maintained, reclassified and deleted systems make no noise in the charts (v1.1 Q46).

mod common;

use common::{TestApp, json_of};
use monita::{
    checks::{self, CheckResult, MonitorInput},
    platform::now,
};
use serde_json::json;

async fn setup(app: &TestApp) -> String {
    let tl = app.tl("rina@x.co").await;
    app.project(&tl, "Payments", "payments").await;
    for name in ["Checkout", "Search"] {
        let m = MonitorInput {
            name: name.into(),
            url: "https://x.invalid/".into(),
            ..MonitorInput::defaults()
        };
        checks::create_monitor(&app.state.db, 1, &m).await.unwrap();
    }
    tl
}

#[tokio::test]
async fn a_failure_inside_a_window_draws_a_band_not_a_failure() {
    let app = TestApp::spawn("test").await;
    let tl = setup(&app).await;
    let t = now();
    app.state
        .db
        .write(async |c| {
            c.execute_batch(&format!(
                "INSERT INTO maintenance_windows (id, project_id, title, message, starts_at, ends_at, created_at) VALUES (1, 1, 'DNS move', '', {s}, {e}, {s});
                 INSERT INTO maintenance_items VALUES (1, 'monitor', 1);",
                s = t - 120,
                e = t + 3600,
            ))
            .await?;
            Ok(())
        })
        .await
        .unwrap();
    let m = checks::find_monitor(&app.state.db, 1, 1)
        .await
        .unwrap()
        .unwrap();
    let failed = CheckResult {
        checked_at: now(),
        ok: false,
        status_code: Some(503),
        latency_ms: 40,
        error: None,
        cert_expires_at: None,
    };
    checks::record_check(&app.state.db, common::BASE_URL, &m, &failed)
        .await
        .unwrap();
    let page = json_of(
        app.get("/api/ui/projects/payments/monitors/1?range=hour", Some(&tl))
            .await,
    )
    .await;
    let chart = &page["data"]["insight"]["chart"];
    assert!(chart["fails"].as_array().unwrap().is_empty(), "{chart}");
    assert_eq!(chart["held"].as_array().unwrap().len(), 1, "{chart}");
    assert!(
        chart["held"][0]["title"]
            .as_str()
            .unwrap()
            .contains("DNS move")
    );
    assert_eq!(page["data"]["maintenance"]["title"], "DNS move");
}

#[tokio::test]
async fn a_deleted_monitor_leaves_the_project_chart_and_the_pulse() {
    let app = TestApp::spawn("test").await;
    let tl = setup(&app).await;
    let t = now();
    app.state
        .db
        .write(async |c| {
            c.execute_batch(&format!(
                "INSERT INTO incidents (id, project_id, origin, title, severity, started_at, resolved_at) VALUES (1, 1, 'auto', 'Search is down', 'critical', {s}, {e});
                 INSERT INTO incident_items VALUES (1, 'monitor', 2);",
                s = t - 1800,
                e = t - 600,
            ))
            .await?;
            Ok(())
        })
        .await
        .unwrap();
    let down = |v: &serde_json::Value| v["data"]["chart"]["down"].as_array().unwrap().len();
    let before = json_of(app.get("/api/ui/projects-chart", Some(&tl)).await).await;
    assert_eq!(down(&before), 1, "{before}");
    let res = app
        .api_send(
            reqwest::Method::DELETE,
            "/api/ui/projects/payments/monitors/2",
            json!({}),
            Some(&tl),
        )
        .await;
    assert!(res.status().is_success());
    let after = json_of(app.get("/api/ui/projects-chart", Some(&tl)).await).await;
    assert_eq!(down(&after), 0, "{after}");
    let list = json_of(app.get("/api/ui/projects", Some(&tl)).await).await;
    let hours = list["data"]["projects"][0]["pulse"]["hours"]
        .as_array()
        .unwrap();
    assert!(hours.iter().all(|h| h["downtime_s"] == 0), "{list}");
}
