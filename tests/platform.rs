mod common;

use common::{RESET_TOKEN, TestApp};
use serde_json::Value;

#[tokio::test]
async fn healthz_reports_ok_once_the_scheduler_ticks() {
    let app = TestApp::spawn("test").await;
    app.state
        .db
        .write(async |c| {
            c.execute(
                "INSERT INTO outbox (channel, recipient, body, next_attempt_at) VALUES ('webhook', 'https://x.invalid/', '{}', unixepoch() + 3600)",
                (),
            )
            .await?;
            Ok(())
        })
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    let res = app.http.get(app.url("/healthz")).send().await.unwrap();
    assert_eq!(res.status(), 200);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["status"], "ok");
    assert_eq!(body["db"], "ok");
    assert_eq!(
        body["outbox_pending"], 1,
        "a row waiting for its retry is still pending"
    );
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
    let wrong = app
        .http
        .post(&url)
        .header("x-reset-token", "test-reset-tokem")
        .json(&serde_json::json!({"seed": "qa"}))
        .send()
        .await
        .unwrap();
    assert_eq!(wrong.status(), 401);
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
    let res = app.http.get(app.url("/api/ui/me")).send().await.unwrap();
    assert_eq!(res.status(), 401);
    assert!(
        res.headers()["content-security-policy"]
            .to_str()
            .unwrap()
            .contains("default-src 'self'")
    );
    assert_eq!(res.headers()["x-content-type-options"], "nosniff");
    assert_eq!(
        res.headers()["cache-control"],
        "no-store",
        "signed-in JSON is never cached"
    );
    let script = app
        .http
        .get(app.url("/static/theme.js"))
        .send()
        .await
        .unwrap();
    assert_eq!(script.status(), 200);
    assert!(
        script.headers()["content-type"]
            .to_str()
            .unwrap()
            .starts_with("text/javascript")
    );
    let missing = app
        .http
        .get(app.url("/static/nope.css"))
        .send()
        .await
        .unwrap();
    assert_eq!(missing.status(), 404);
}

#[tokio::test]
async fn a_page_path_gets_the_frontend_and_an_unknown_api_path_gets_json() {
    let app = TestApp::spawn("test").await;
    let page = app
        .http
        .get(app.url("/a/page/no/route/claims"))
        .send()
        .await
        .unwrap();
    // 503 names `make web` when the frontend was not built before the test binary.
    assert!(
        matches!(page.status().as_u16(), 200 | 503),
        "{}",
        page.status()
    );
    let api = app.http.get(app.url("/api/ui/nope")).send().await.unwrap();
    assert_eq!(api.status(), 404);
    assert_eq!(common::json_of(api).await["error"]["code"], "not_found");
}

#[tokio::test]
async fn a_body_over_128_kib_is_refused() {
    let app = TestApp::spawn("test").await;
    let login = |padding: usize| {
        app.http
            .post(app.url("/api/ui/login"))
            .header("origin", common::BASE_URL)
            .json(&serde_json::json!({ "email": "a@x.co", "password": "x".repeat(padding) }))
            .send()
    };
    // A ticket of 10 000 emoji, percent-encoded, is about 120 KB and must still fit.
    assert_eq!(login(120_000).await.unwrap().status(), 422);
    assert_eq!(login(200_000).await.unwrap().status(), 413);
}

#[tokio::test]
async fn json_goes_out_as_zstd_to_its_own_pages_and_plain_to_a_cross_site_request() {
    let app = TestApp::spawn("test").await;
    let cookie = app.super_admin("root@example.com").await;
    let get = |site: &'static str| {
        app.http
            .get(app.url("/api/ui/admin/system"))
            .header("cookie", &cookie)
            .header("accept-encoding", "gzip, br, zstd")
            .header("sec-fetch-site", site)
            .send()
    };
    let packed = get("same-origin").await.unwrap();
    assert_eq!(packed.status(), 200);
    assert_eq!(packed.headers()["content-encoding"], "zstd");
    assert_eq!(packed.headers()["vary"], "accept-encoding");
    let body = zstd::decode_all(&packed.bytes().await.unwrap()[..]).unwrap();
    let plain = get("cross-site").await.unwrap();
    assert!(plain.headers().get("content-encoding").is_none());
    let plain: Value = plain.json().await.unwrap();
    assert!(
        plain.to_string().len() > 1024,
        "the body is big enough to compress"
    );
    assert_eq!(serde_json::from_slice::<Value>(&body).unwrap(), plain);
}
