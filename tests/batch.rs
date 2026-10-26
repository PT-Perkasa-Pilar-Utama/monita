//! Pause, resume or delete many monitors or heartbeats at once: all or nothing.

mod common;

use common::{TestApp, json_of};
use serde_json::{Value, json};

fn monitor(name: &str) -> Value {
    json!({ "name": name, "url": "http://127.0.0.1:9/", "interval_s": "60", "timeout_s": "5",
            "expected_status": "2xx", "severity": "warning" })
}

async fn setup(app: &TestApp) -> (String, Vec<i64>) {
    let tl = app.tl("rina@x.co").await;
    app.project(&tl, "Payments", "payments").await;
    let mut ids = Vec::new();
    for name in ["API", "Web", "Admin"] {
        let res = app
            .api_post(
                "/api/ui/projects/payments/monitors",
                monitor(name),
                Some(&tl),
            )
            .await;
        assert_eq!(res.status(), 201);
        ids.push(json_of(res).await["data"]["id"].as_i64().unwrap());
    }
    (tl, ids)
}

async fn batch(app: &TestApp, tl: &str, kind: &str, body: Value) -> reqwest::Response {
    app.api_post(
        &format!("/api/ui/projects/payments/{kind}/batch"),
        body,
        Some(tl),
    )
    .await
}

async fn paused(app: &TestApp, tl: &str) -> Vec<bool> {
    let list = json_of(
        app.get("/api/ui/projects/payments/monitors", Some(tl))
            .await,
    )
    .await;
    let mut rows: Vec<(i64, bool)> = list["data"]["monitors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| (m["id"].as_i64().unwrap(), !m["paused_at"].is_null()))
        .collect();
    rows.sort();
    rows.into_iter().map(|(_, p)| p).collect()
}

#[tokio::test]
async fn a_batch_pauses_and_resumes_the_picked_monitors() {
    let app = TestApp::spawn("test").await;
    let (tl, ids) = setup(&app).await;
    let res = batch(
        &app,
        &tl,
        "monitors",
        json!({ "action": "pause", "ids": [ids[0], ids[2], ids[0]] }),
    )
    .await;
    assert_eq!(res.status(), 200);
    assert_eq!(
        json_of(res).await["data"]["count"],
        2,
        "a repeated id counts once"
    );
    assert_eq!(paused(&app, &tl).await, [true, false, true]);
    let res = batch(
        &app,
        &tl,
        "monitors",
        json!({ "action": "resume", "ids": [ids[0], ids[2]] }),
    )
    .await;
    assert_eq!(res.status(), 200);
    assert_eq!(paused(&app, &tl).await, [false, false, false]);
}

#[tokio::test]
async fn one_unknown_id_refuses_the_whole_batch() {
    let app = TestApp::spawn("test").await;
    let (tl, ids) = setup(&app).await;
    let res = batch(
        &app,
        &tl,
        "monitors",
        json!({ "action": "pause", "ids": [ids[0], 9999] }),
    )
    .await;
    assert_eq!(res.status(), 404);
    assert_eq!(
        paused(&app, &tl).await,
        [false, false, false],
        "nothing changed"
    );
    let res = batch(
        &app,
        &tl,
        "monitors",
        json!({ "action": "delete", "ids": [ids[1], 9999] }),
    )
    .await;
    assert_eq!(res.status(), 404);
    assert_eq!(paused(&app, &tl).await.len(), 3, "nothing deleted");
}

#[tokio::test]
async fn a_batch_deletes_monitors_and_heartbeats() {
    let app = TestApp::spawn("test").await;
    let (tl, ids) = setup(&app).await;
    let res = batch(
        &app,
        &tl,
        "monitors",
        json!({ "action": "delete", "ids": [ids[0], ids[1]] }),
    )
    .await;
    assert_eq!(res.status(), 200);
    assert_eq!(paused(&app, &tl).await.len(), 1);
    let hb = app
        .api_post(
            "/api/ui/projects/payments/heartbeats",
            json!({ "name": "nightly", "period_s": "3600", "grace_s": "300", "severity": "warning" }),
            Some(&tl),
        )
        .await;
    let hb = json_of(hb).await["data"]["id"].as_i64().unwrap();
    let res = batch(
        &app,
        &tl,
        "heartbeats",
        json!({ "action": "delete", "ids": [hb] }),
    )
    .await;
    assert_eq!(res.status(), 200);
    let list = json_of(
        app.get("/api/ui/projects/payments/heartbeats", Some(&tl))
            .await,
    )
    .await;
    assert!(list["data"]["heartbeats"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn a_bad_batch_names_its_field() {
    let app = TestApp::spawn("test").await;
    let (tl, ids) = setup(&app).await;
    let res = batch(
        &app,
        &tl,
        "monitors",
        json!({ "action": "archive", "ids": [ids[0]] }),
    )
    .await;
    assert_eq!(res.status(), 422);
    assert!(json_of(res).await["error"]["fields"]["action"].is_string());
    let res = batch(
        &app,
        &tl,
        "monitors",
        json!({ "action": "pause", "ids": [] }),
    )
    .await;
    assert_eq!(res.status(), 422);
    assert!(json_of(res).await["error"]["fields"]["ids"].is_string());
    let many: Vec<i64> = (1..=101).collect();
    let res = batch(
        &app,
        &tl,
        "monitors",
        json!({ "action": "pause", "ids": many }),
    )
    .await;
    assert_eq!(res.status(), 422, "over the per-project cap");
}

#[tokio::test]
async fn a_write_key_may_batch_and_a_read_key_may_not() {
    let app = TestApp::spawn("test").await;
    let (tl, ids) = setup(&app).await;
    let key = |scope: &'static str| {
        app.api_post(
            "/api/ui/projects/payments/keys",
            json!({ "name": scope, "scope": scope }),
            Some(&tl),
        )
    };
    let read = json_of(key("read").await).await["data"]["key"]
        .as_str()
        .unwrap()
        .to_string();
    let write = json_of(key("write").await).await["data"]["key"]
        .as_str()
        .unwrap()
        .to_string();
    let call = |k: String| {
        app.http
            .post(app.url("/api/v1/projects/payments/monitors/batch"))
            .bearer_auth(k)
            .json(&json!({ "action": "pause", "ids": [ids[0]] }))
            .send()
    };
    assert_eq!(call(read).await.unwrap().status(), 403);
    assert_eq!(call(write).await.unwrap().status(), 200);
}
