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
    app.http
        .post(app.url("/api/v1/ref-ids"))
        .bearer_auth(key)
        .send()
        .await
        .unwrap()
        .status()
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
