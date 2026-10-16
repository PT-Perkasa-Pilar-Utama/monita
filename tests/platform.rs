mod common;

use common::{RESET_TOKEN, TestApp};
use serde_json::Value;

#[tokio::test]
async fn healthz_reports_ok_once_the_scheduler_ticks() {
    let app = TestApp::spawn("test").await;
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    let res = app.http.get(app.url("/healthz")).send().await.unwrap();
    assert_eq!(res.status(), 200);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "ok");
    assert_eq!(body["db"], "ok");
    assert_eq!(body["outbox_pending"], 0);
}

#[tokio::test]
async fn reset_state_needs_the_token_and_reseeds() {
    let app = TestApp::spawn("test").await;
    let url = app.url("/admin/reset-state");
    let no_token = app
        .http
        .post(&url)
        .json(&serde_json::json!({"seed": "qa"}))
        .send()
        .await
        .unwrap();
    assert_eq!(no_token.status(), 401);
    let ok = app
        .http
        .post(&url)
        .header("x-reset-token", RESET_TOKEN)
        .json(&serde_json::json!({"seed": "qa"}))
        .send()
        .await
        .unwrap();
    assert_eq!(ok.status(), 204);
    let bad_seed = app
        .http
        .post(&url)
        .header("x-reset-token", RESET_TOKEN)
        .json(&serde_json::json!({"seed": "prod"}))
        .send()
        .await
        .unwrap();
    assert_eq!(bad_seed.status(), 400);
}

#[tokio::test]
async fn reset_state_does_not_exist_in_prod() {
    let app = TestApp::spawn("prod").await;
    let res = app
        .http
        .post(app.url("/admin/reset-state"))
        .header("x-reset-token", RESET_TOKEN)
        .json(&serde_json::json!({"seed": "qa"}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 404, "SEC-17");
}

#[tokio::test]
async fn pages_carry_security_headers_and_assets_load() {
    let app = TestApp::spawn("test").await;
    let res = app.http.get(app.url("/login")).send().await.unwrap();
    assert_eq!(res.status(), 200);
    assert!(
        res.headers()["content-security-policy"]
            .to_str()
            .unwrap()
            .contains("default-src 'self'")
    );
    assert_eq!(res.headers()["x-content-type-options"], "nosniff");
    let css = app
        .http
        .get(app.url("/static/app.css"))
        .send()
        .await
        .unwrap();
    assert_eq!(css.status(), 200);
    assert!(
        css.headers()["content-type"]
            .to_str()
            .unwrap()
            .starts_with("text/css")
    );
    let missing = app
        .http
        .get(app.url("/static/nope.css"))
        .send()
        .await
        .unwrap();
    assert_eq!(missing.status(), 404);
}
