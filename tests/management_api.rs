//! Project API keys with a scope (v1.1 Q44): `ref_ids`, `read` or `write`.

mod common;

use common::{TestApp, json_of};
use serde_json::{Value, json};

/// Creates a key with `scope` through the frontend JSON and returns the full key.
async fn key(app: &TestApp, cookie: &str, slug: &str, scope: &str) -> String {
    let res = app
        .api_post(
            &format!("/api/ui/projects/{slug}/keys"),
            json!({ "name": format!("{scope}-key"), "scope": scope }),
            Some(cookie),
        )
        .await;
    assert_eq!(res.status(), 201);
    json_of(res).await["data"]["key"]
        .as_str()
        .unwrap()
        .to_string()
}

async fn ref_id(app: &TestApp, key: &str) -> reqwest::StatusCode {
    let res = app
        .http
        .post(app.url("/api/v1/ref-ids"))
        .bearer_auth(key)
        .send()
        .await
        .unwrap();
    if res.status() == 201 {
        assert_eq!(
            res.headers()["cache-control"],
            "no-store",
            "a ref ID is never cached"
        );
    }
    res.status()
}

#[tokio::test]
async fn a_key_keeps_the_scope_it_was_created_with() {
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    app.project(&tl, "Payments", "payments").await;
    key(&app, &tl, "payments", "read").await;
    key(&app, &tl, "payments", "write").await;
    let plain = app
        .api_post(
            "/api/ui/projects/payments/keys",
            json!({ "name": "old-client" }),
            Some(&tl),
        )
        .await;
    assert_eq!(plain.status(), 201, "no scope means ref IDs only");
    let list = json_of(app.get("/api/ui/projects/payments/keys", Some(&tl)).await).await;
    let mut scopes: Vec<(String, String)> = list["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|k| {
            (
                k["name"].as_str().unwrap().into(),
                k["scope"].as_str().unwrap().into(),
            )
        })
        .collect();
    scopes.sort();
    assert_eq!(
        scopes,
        [
            ("old-client".into(), "ref_ids".into()),
            ("read-key".into(), "read".into()),
            ("write-key".into(), "write".into()),
        ]
    );
    let bad = app
        .api_post(
            "/api/ui/projects/payments/keys",
            json!({ "name": "x", "scope": "admin" }),
            Some(&tl),
        )
        .await;
    assert_eq!(bad.status(), 422);
    let err: Value = json_of(bad).await;
    assert!(err["error"]["fields"]["scope"].is_string(), "{err}");
}

#[tokio::test]
async fn a_read_key_cannot_issue_ref_ids_and_a_write_key_can() {
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    app.project(&tl, "Payments", "payments").await;
    let read = key(&app, &tl, "payments", "read").await;
    let write = key(&app, &tl, "payments", "write").await;
    let ref_ids = key(&app, &tl, "payments", "ref_ids").await;
    assert_eq!(ref_id(&app, &read).await, 403);
    assert_eq!(ref_id(&app, &write).await, 201);
    assert_eq!(ref_id(&app, &ref_ids).await, 201);
}

/// A call to the key API with `key`, as a project's own server makes it: no cookie, no origin.
async fn call(
    app: &TestApp,
    method: reqwest::Method,
    path: &str,
    key: &str,
    body: Option<Value>,
) -> reqwest::Response {
    let mut req = app
        .http
        .request(method, app.url(&format!("/api/v1/projects{path}")))
        .bearer_auth(key);
    if let Some(b) = body {
        req = req.json(&b);
    }
    req.send().await.unwrap()
}

async fn get(app: &TestApp, path: &str, key: &str) -> reqwest::Response {
    call(app, reqwest::Method::GET, path, key, None).await
}

async fn post(app: &TestApp, path: &str, key: &str, body: Value) -> reqwest::Response {
    call(app, reqwest::Method::POST, path, key, Some(body)).await
}

fn monitor_body() -> Value {
    json!({ "name": "API", "url": "http://127.0.0.1:9/", "interval_s": "60", "timeout_s": "5",
            "expected_status": "2xx", "severity": "warning", "method": "POST",
            "headers": "X-Key: s3cret", "body": "{\"token\":\"t0p\"}" })
}

fn heartbeat_body() -> Value {
    json!({ "name": "nightly", "period_s": "3600", "grace_s": "300", "severity": "warning" })
}

#[tokio::test]
async fn a_write_key_manages_its_project_and_writes_as_itself() {
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    app.project(&tl, "Payments", "payments").await;
    let write = key(&app, &tl, "payments", "write").await;

    let made = post(&app, "/payments/monitors", &write, monitor_body()).await;
    assert_eq!(made.status(), 201);
    let id = json_of(made).await["data"]["id"].as_i64().unwrap();
    let seen = json_of(get(&app, &format!("/payments/monitors/{id}"), &write).await).await;
    assert_eq!(seen["data"]["monitor"]["headers"], "X-Key: s3cret");

    let hb = post(&app, "/payments/heartbeats", &write, heartbeat_body()).await;
    assert_eq!(hb.status(), 201);
    let hb_id = json_of(hb).await["data"]["id"].as_i64().unwrap();
    let hb = get(&app, &format!("/payments/heartbeats/{hb_id}"), &write).await;
    assert_eq!(
        hb.headers()["cache-control"],
        "no-store",
        "a ping URL is never cached"
    );
    let hb = json_of(hb).await;
    assert!(hb["data"]["ping_url"].as_str().unwrap().contains("/ping/"));

    let paused = call(
        &app,
        reqwest::Method::PUT,
        &format!("/payments/heartbeats/{hb_id}/paused"),
        &write,
        Some(json!({ "paused": true })),
    )
    .await;
    assert!(paused.status().is_success(), "{}", paused.status());

    let opened = post(
        &app,
        "/payments/incidents",
        &write,
        json!({ "title": "Slow checkout", "items": [format!("monitor:{id}")], "severity": "warning",
                "message": "Deploy bot is rolling back." }),
    )
    .await;
    assert_eq!(opened.status(), 201);
    let incident = json_of(opened).await["data"]["id"].as_i64().unwrap();
    let done = post(
        &app,
        &format!("/payments/incidents/{incident}/updates"),
        &write,
        json!({ "label": "resolved", "message": "Rolled back." }),
    )
    .await;
    assert_eq!(done.status(), 204);
    let team = json_of(
        app.api_get(
            &format!("/api/ui/projects/payments/incidents/{incident}"),
            Some(&tl),
        )
        .await,
    )
    .await;
    for u in team["data"]["updates"].as_array().unwrap() {
        assert_eq!(u["author_key"], "write-key", "{team}");
    }

    let t = monita::platform::now();
    let window = post(
        &app,
        "/payments/maintenance",
        &write,
        json!({ "title": "Network move", "starts_at": t + 7200, "ends_at": t + 9000,
                "items": [format!("monitor:{id}")] }),
    )
    .await;
    assert_eq!(window.status(), 201);
    let project = json_of(get(&app, "/payments", &write).await).await;
    assert_eq!(project["data"]["project"]["slug"], "payments");
}

#[tokio::test]
async fn a_read_key_reads_without_secrets_and_changes_nothing() {
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    app.project(&tl, "Payments", "payments").await;
    let made = app
        .api_post(
            "/api/ui/projects/payments/monitors",
            monitor_body(),
            Some(&tl),
        )
        .await;
    let id = json_of(made).await["data"]["id"].as_i64().unwrap();
    let hb = app
        .api_post(
            "/api/ui/projects/payments/heartbeats",
            heartbeat_body(),
            Some(&tl),
        )
        .await;
    let hb_id = json_of(hb).await["data"]["id"].as_i64().unwrap();
    let read = key(&app, &tl, "payments", "read").await;

    let seen = json_of(get(&app, &format!("/payments/monitors/{id}"), &read).await).await;
    assert_eq!(seen["data"]["monitor"]["headers"], "X-Key: (hidden)");
    assert_eq!(seen["data"]["monitor"]["body"], "(hidden)");
    let hb = json_of(get(&app, &format!("/payments/heartbeats/{hb_id}"), &read).await).await;
    assert!(hb["data"]["ping_url"].is_null(), "{hb}");
    assert_eq!(get(&app, "/payments/incidents", &read).await.status(), 200);
    assert_eq!(
        get(&app, "/payments/maintenance", &read).await.status(),
        200
    );

    let refused = post(&app, "/payments/heartbeats", &read, heartbeat_body()).await;
    assert_eq!(refused.status(), 403);
    let gone = call(
        &app,
        reqwest::Method::DELETE,
        &format!("/payments/monitors/{id}"),
        &read,
        None,
    )
    .await;
    assert_eq!(gone.status(), 403);
}

#[tokio::test]
async fn a_key_reaches_only_its_own_project_and_only_the_listed_routes() {
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    app.project(&tl, "Payments", "payments").await;
    app.project(&tl, "Search", "search").await;
    let write = key(&app, &tl, "payments", "write").await;
    let ref_ids = key(&app, &tl, "payments", "ref_ids").await;

    for path in ["/search", "/search/monitors", "/nope/monitors"] {
        assert_eq!(get(&app, path, &write).await.status(), 404, "{path}");
    }
    assert_eq!(
        get(&app, "/payments/monitors", &ref_ids).await.status(),
        403
    );
    assert_eq!(
        get(&app, "/payments/monitors", "mk_wrong").await.status(),
        401
    );

    // Keys, secrets, alert cards, the Telegram link, tickets and delete stay off the key API.
    for path in [
        "/payments/keys",
        "/payments/secrets",
        "/payments/alert-cards",
        "/payments/tickets",
    ] {
        assert_eq!(get(&app, path, &write).await.status(), 404, "{path}");
    }
    for path in [
        "/payments/keys",
        "/payments/telegram-link",
        "/payments/ref-ids",
    ] {
        let res = post(&app, path, &write, json!({ "name": "x" })).await;
        assert_eq!(res.status(), 404, "{path}");
    }
    let delete = call(
        &app,
        reqwest::Method::DELETE,
        "/payments",
        &write,
        Some(json!({ "confirm": "Payments" })),
    )
    .await;
    assert_eq!(delete.status(), 405, "the project read is GET only");
}

#[tokio::test]
async fn a_key_and_a_session_each_work_only_on_their_own_api() {
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    app.project(&tl, "Payments", "payments").await;
    let write = key(&app, &tl, "payments", "write").await;
    let with_key = app
        .http
        .get(app.url("/api/ui/projects/payments/monitors"))
        .bearer_auth(&write)
        .send()
        .await
        .unwrap();
    assert_eq!(with_key.status(), 401);
    let with_cookie = app
        .get("/api/v1/projects/payments/monitors", Some(&tl))
        .await;
    assert_eq!(with_cookie.status(), 401);

    let list = json_of(app.get("/api/ui/projects/payments/keys", Some(&tl)).await).await;
    let id = list["data"][0]["id"].as_i64().unwrap();
    let revoked = app
        .api_post(
            &format!("/api/ui/projects/payments/keys/{id}/revoke"),
            json!({}),
            Some(&tl),
        )
        .await;
    assert!(revoked.status().is_success());
    assert_eq!(get(&app, "/payments/monitors", &write).await.status(), 401);
}
