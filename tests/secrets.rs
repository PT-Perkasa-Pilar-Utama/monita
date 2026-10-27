//! Sealed project secrets: written once, listed by name, never returned.

mod common;

use common::{TestApp, json_of};
use serde_json::json;

const PATH: &str = "/api/ui/projects/payments/secrets";

#[tokio::test]
async fn a_secret_is_written_once_and_never_comes_back() {
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    app.project(&tl, "Payments", "payments").await;
    let other = app.tl("budi@x.co").await;

    let body = json!({ "name": "REPORT_TOKEN", "value": "s3cr3t-value" });
    assert_eq!(
        app.api_post(PATH, body.clone(), Some(&other))
            .await
            .status(),
        404,
        "owner only"
    );
    let res = app.api_post(PATH, body.clone(), Some(&tl)).await;
    assert_eq!(res.status(), 201);
    let text = res.text().await.unwrap();
    assert!(!text.contains("s3cr3t"), "{text}");

    let dup = app.api_post(PATH, body, Some(&tl)).await;
    assert_eq!(dup.status(), 422);
    assert!(json_of(dup).await["error"]["fields"]["name"].is_string());
    let bad = app
        .api_post(
            PATH,
            json!({ "name": "has space", "value": "x" }),
            Some(&tl),
        )
        .await;
    assert_eq!(bad.status(), 422);

    let list = app.api_get(PATH, Some(&tl)).await.text().await.unwrap();
    assert!(!list.contains("s3cr3t"), "{list}");
    let rows: serde_json::Value = serde_json::from_str(&list).unwrap();
    assert_eq!(rows["data"][0]["name"], "REPORT_TOKEN");
    assert_eq!(rows["data"][0]["used_by"], 0);
    let id = rows["data"][0]["id"].as_i64().unwrap();

    let one = format!("{PATH}/{id}");
    let replaced = app
        .api_send(
            reqwest::Method::PUT,
            &one,
            json!({ "value": "n3w" }),
            Some(&tl),
        )
        .await;
    assert_eq!(replaced.status(), 204);
    let empty = app
        .api_send(
            reqwest::Method::PUT,
            &one,
            json!({ "value": "" }),
            Some(&tl),
        )
        .await;
    assert_eq!(empty.status(), 422);
    let gone = app
        .api_send(reqwest::Method::DELETE, &one, json!({}), Some(&tl))
        .await;
    assert_eq!(gone.status(), 204);
    let again = app
        .api_send(reqwest::Method::DELETE, &one, json!({}), Some(&tl))
        .await;
    assert_eq!(again.status(), 404);
}

#[tokio::test]
async fn another_projects_secret_in_use_is_not_found_and_not_counted() {
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    app.project(&tl, "Payments", "payments").await;
    let other = app.tl("budi@x.co").await;
    app.project(&other, "Search", "search").await;
    let id = json_of(
        app.api_post(
            PATH,
            json!({ "name": "API_TOKEN", "value": "v" }),
            Some(&tl),
        )
        .await,
    )
    .await["data"]["id"]
        .as_i64()
        .unwrap();
    let monitor = app
        .api_post(
            "/api/ui/projects/payments/monitors",
            json!({ "name": "Checkout", "kind": "http", "url": "http://127.0.0.1:9/",
                    "interval_s": "60", "timeout_s": "10", "expected_status": "2xx",
                    "severity": "critical", "method": "GET",
                    "auth_kind": "bearer", "auth_secret_id": id }),
            Some(&tl),
        )
        .await;
    assert_eq!(monitor.status(), 201);
    let foreign = app
        .api_send(
            reqwest::Method::DELETE,
            &format!("/api/ui/projects/search/secrets/{id}"),
            json!({}),
            Some(&other),
        )
        .await;
    assert_eq!(
        foreign.status(),
        404,
        "the same answer as a missing id, so it says nothing about another project"
    );
    let own = app
        .api_send(
            reqwest::Method::DELETE,
            &format!("{PATH}/{id}"),
            json!({}),
            Some(&tl),
        )
        .await;
    assert_eq!(own.status(), 422, "the owner still learns it is in use");
}
