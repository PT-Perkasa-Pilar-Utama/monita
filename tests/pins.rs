//! Pins (decision Q48): each user's favourite projects, monitors and heartbeats lead their lists
//! and the charts. At most 6 of each, per user, and per project for monitors and heartbeats.

mod common;

use common::{BASE_URL, TestApp, json_of};
use monita::platform::db::query_i64;
use reqwest::Method;
use serde_json::{Value, json};

async fn pin(app: &TestApp, cookie: &str, path: &str, on: bool) -> reqwest::StatusCode {
    let method = if on { Method::PUT } else { Method::DELETE };
    app.api_send(method, path, json!({}), Some(cookie))
        .await
        .status()
}

async fn get(app: &TestApp, cookie: &str, path: &str) -> Value {
    let res = app.api_get(path, Some(cookie)).await;
    assert_eq!(res.status(), 200, "{path}");
    json_of(res).await["data"].clone()
}

async fn monitor(app: &TestApp, cookie: &str, slug: &str, name: &str) -> i64 {
    let res = app
        .api_post(
            &format!("/api/ui/projects/{slug}/monitors"),
            json!({ "name": name, "url": "https://x.invalid/", "interval_s": "60",
                    "timeout_s": "10", "expected_status": "2xx", "severity": "warning" }),
            Some(cookie),
        )
        .await;
    assert_eq!(res.status(), 201);
    json_of(res).await["data"]["id"].as_i64().unwrap()
}

async fn rows(app: &TestApp, sql: &str) -> i64 {
    query_i64(&app.state.db.read().await.unwrap(), sql, ())
        .await
        .unwrap()
        .unwrap_or(0)
}

#[tokio::test]
async fn six_projects_pin_a_seventh_waits_and_pinned_ones_lead_the_list() {
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    for n in 1..=7 {
        app.project(&tl, &format!("Project {n}"), &format!("p{n}"))
            .await;
    }
    for n in 2..=7 {
        assert_eq!(
            pin(&app, &tl, &format!("/api/ui/projects/p{n}/pin"), true).await,
            204
        );
    }
    let full = app
        .api_send(Method::PUT, "/api/ui/projects/p1/pin", json!({}), Some(&tl))
        .await;
    assert_eq!(full.status(), 422);
    assert_eq!(
        json_of(full).await["error"]["message"],
        "6 projects are pinned, which is the most, so unpin one to pin Project 1."
    );
    assert_eq!(
        pin(&app, &tl, "/api/ui/projects/p7/pin", true).await,
        204,
        "a pin twice is one pin"
    );
    assert_eq!(pin(&app, &tl, "/api/ui/projects/p7/pin", false).await, 204);
    assert_eq!(pin(&app, &tl, "/api/ui/projects/p1/pin", true).await, 204);

    let list = get(&app, &tl, "/api/ui/projects?sort=name").await;
    let order: Vec<(String, bool)> = list["projects"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            (
                p["slug"].as_str().unwrap().to_string(),
                p["pinned"].as_bool().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        order[6],
        ("p7".to_string(), false),
        "the unpinned one comes after the pins"
    );
    assert!(order[..6].iter().all(|(_, pinned)| *pinned));
    assert_eq!(
        order[0].0, "p1",
        "the chosen sort orders the pins among themselves"
    );
}

#[tokio::test]
async fn pins_are_personal_a_supervisor_pins_and_a_non_owner_or_a_key_cannot() {
    let app = TestApp::spawn("test").await;
    let admin = app.super_admin("root@x.co").await;
    let tl = app.tl("rina@x.co").await;
    let other = app.tl("budi@x.co").await;
    app.project(&tl, "Payments", "payments").await;
    let res = app
        .api_post(
            "/api/ui/admin/users",
            json!({ "email": "sari@x.co", "role": "supervisor" }),
            Some(&admin),
        )
        .await;
    let link = json_of(res).await["data"]["link"]
        .as_str()
        .unwrap()
        .to_string();
    let supervisor = app
        .accept(link.strip_prefix(BASE_URL).unwrap(), common::PASSWORD)
        .await;

    assert_eq!(
        pin(&app, &supervisor, "/api/ui/projects/payments/pin", true).await,
        204
    );
    assert_eq!(
        get(&app, &supervisor, "/api/ui/projects").await["projects"][0]["pinned"],
        true
    );
    assert_eq!(
        get(&app, &tl, "/api/ui/projects").await["projects"][0]["pinned"],
        false,
        "another user's pin does not show"
    );
    assert_eq!(
        pin(&app, &other, "/api/ui/projects/payments/pin", true).await,
        404
    );

    let res = app
        .api_post(
            "/api/ui/projects/payments/keys",
            json!({ "name": "w", "scope": "write" }),
            Some(&tl),
        )
        .await;
    let key = json_of(res).await["data"]["key"]
        .as_str()
        .unwrap()
        .to_string();
    let m = monitor(&app, &tl, "payments", "Checkout").await;
    for path in [
        "/api/v1/projects/payments/pin".to_string(),
        format!("/api/v1/projects/payments/monitors/{m}/pin"),
    ] {
        let res = app
            .http
            .put(app.url(&path))
            .bearer_auth(&key)
            .send()
            .await
            .unwrap();
        assert!(
            res.status() == 404 || res.status() == 405,
            "{path}: {}",
            res.status()
        );
    }
    let listed = app
        .http
        .get(app.url("/api/v1/projects/payments/monitors"))
        .bearer_auth(&key)
        .send()
        .await
        .unwrap();
    assert_eq!(
        json_of(listed).await["data"]["monitors"][0]["pinned"],
        false
    );
}

#[tokio::test]
async fn monitor_and_heartbeat_pins_lead_their_lists_and_the_overview_and_go_with_them() {
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    app.project(&tl, "Payments", "payments").await;
    let mut ids = Vec::new();
    for n in 1..=7 {
        ids.push(monitor(&app, &tl, "payments", &format!("Service {n}")).await);
    }
    for id in &ids[..6] {
        assert_eq!(
            pin(
                &app,
                &tl,
                &format!("/api/ui/projects/payments/monitors/{id}/pin"),
                true
            )
            .await,
            204
        );
    }
    let full = app
        .api_send(
            Method::PUT,
            &format!("/api/ui/projects/payments/monitors/{}/pin", ids[6]),
            json!({}),
            Some(&tl),
        )
        .await;
    assert_eq!(full.status(), 422);
    assert_eq!(
        json_of(full).await["error"]["message"],
        "6 monitors are pinned in this project, which is the most, so unpin one to pin Service 7."
    );
    assert_eq!(
        pin(
            &app,
            &tl,
            "/api/ui/projects/payments/monitors/9999/pin",
            true
        )
        .await,
        404
    );

    let list = get(&app, &tl, "/api/ui/projects/payments/monitors").await;
    let pinned = list["monitors"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|m| m["pinned"] == true)
        .count();
    assert_eq!(pinned, 6);
    let chart = get(&app, &tl, "/api/ui/projects/payments/overview").await["chart"].clone();
    let lines = chart["lines"].as_array().unwrap();
    assert_eq!(lines.len(), 6, "every pin gets a line");
    assert!(lines.iter().all(|l| l["pinned"] == true));

    let res = app
        .api_post(
            "/api/ui/projects/payments/heartbeats",
            json!({ "name": "nightly", "period_s": "3600", "grace_s": "300", "severity": "warning" }),
            Some(&tl),
        )
        .await;
    let hb = json_of(res).await["data"]["id"].as_i64().unwrap();
    assert_eq!(
        pin(
            &app,
            &tl,
            &format!("/api/ui/projects/payments/heartbeats/{hb}/pin"),
            true
        )
        .await,
        204
    );
    assert_eq!(
        get(&app, &tl, "/api/ui/projects/payments/heartbeats").await["heartbeats"][0]["pinned"],
        true
    );

    let res = app
        .api_send(
            Method::DELETE,
            &format!("/api/ui/projects/payments/monitors/{}", ids[0]),
            json!({}),
            Some(&tl),
        )
        .await;
    assert!(res.status().is_success());
    assert_eq!(
        rows(&app, "SELECT count(*) FROM monitor_pins").await,
        5,
        "a deleted monitor takes its pin"
    );
    assert_eq!(
        pin(
            &app,
            &tl,
            &format!("/api/ui/projects/payments/monitors/{}/pin", ids[6]),
            true
        )
        .await,
        204,
        "and frees the place"
    );
}
