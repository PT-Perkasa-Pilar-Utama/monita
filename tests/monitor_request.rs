//! A monitor sends its method, headers and body, and the header values stay out of check errors
//! and the public status page.

mod common;

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::{Router, body::Bytes, extract::State, http::HeaderMap, routing::any};
use common::{TestApp, json_of};
use monita::platform::db::query_i64;
use serde_json::{Value, json};

/// (method, Authorization header, body, X-Api-Key header) of each request the server got.
type Seen = Arc<Mutex<Vec<(String, String, String, String)>>>;

async fn recorder() -> (String, Seen) {
    let seen: Seen = Arc::default();
    let app =
        Router::new()
            .route(
                "/health",
                any(
                    |State(s): State<Seen>,
                     method: axum::http::Method,
                     h: HeaderMap,
                     body: Bytes| async move {
                        let header = |name| {
                            h.get(name)
                                .and_then(|v| v.to_str().ok())
                                .unwrap_or("")
                                .to_string()
                        };
                        s.lock().unwrap().push((
                            method.to_string(),
                            header("authorization"),
                            String::from_utf8_lossy(&body).to_string(),
                            header("x-api-key"),
                        ));
                        "ok"
                    },
                ),
            )
            .with_state(seen.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (format!("http://{addr}/health"), seen)
}

async fn count(app: &TestApp, sql: &str) -> i64 {
    query_i64(&app.state.db.read().await.unwrap(), sql, ())
        .await
        .unwrap()
        .unwrap_or(0)
}

/// Makes every monitor due and waits for `n` check results in all.
async fn checks(app: &TestApp, n: i64) {
    app.state
        .db
        .write(async |c| {
            c.execute("UPDATE monitors SET next_check_at = 0", ())
                .await?;
            Ok(())
        })
        .await
        .unwrap();
    for _ in 0..50 {
        if count(app, "SELECT count(*) FROM check_results").await >= n {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("no check ran");
}

fn form(url: &str, method: &str, headers: &str, body: &str) -> Value {
    json!({ "name": "API", "url": url, "interval_s": "60", "timeout_s": "5", "expected_status": "2xx",
            "severity": "warning", "is_public": true, "method": method, "headers": headers, "body": body })
}

#[tokio::test]
async fn a_monitor_sends_its_method_headers_and_body() {
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    app.project(&tl, "Payments", "payments").await;
    let path = "/api/ui/projects/payments/monitors";

    let get_body = app
        .api_post(
            path,
            form("http://127.0.0.1:9/", "GET", "", "{}"),
            Some(&tl),
        )
        .await;
    assert_eq!(get_body.status(), 422);
    assert!(json_of(get_body).await["error"]["fields"]["body"].is_string());
    let host = app
        .api_post(
            path,
            form("http://127.0.0.1:9/", "GET", "Host: x", ""),
            Some(&tl),
        )
        .await;
    assert!(json_of(host).await["error"]["fields"]["headers"].is_string());

    let (url, seen) = recorder().await;
    let res = app
        .api_post(
            path,
            form(
                &url,
                "POST",
                "Authorization: Bearer t0ken\nX-Env: prod",
                r#"{"ping":1}"#,
            ),
            Some(&tl),
        )
        .await;
    assert_eq!(res.status(), 201);
    checks(&app, 1).await;
    let got = seen.lock().unwrap().clone();
    assert_eq!(
        got[0],
        (
            "POST".into(),
            "Bearer t0ken".into(),
            r#"{"ping":1}"#.into(),
            String::new()
        )
    );
    let detail = json_of(app.api_get(&format!("{path}/1"), Some(&tl)).await).await;
    assert_eq!(detail["data"]["monitor"]["method"], "POST");

    // A failing check: the error text and the status page never carry the token.
    let closed = form(
        "http://127.0.0.1:9/",
        "POST",
        "Authorization: Bearer t0ken",
        "{}",
    );
    assert_eq!(
        app.api_send(
            reqwest::Method::PUT,
            &format!("{path}/1"),
            closed,
            Some(&tl)
        )
        .await
        .status(),
        204
    );
    checks(&app, 2).await;
    assert_eq!(
        count(
            &app,
            "SELECT count(*) FROM check_results WHERE error LIKE '%t0ken%'"
        )
        .await,
        0
    );
    assert_eq!(
        count(&app, "SELECT count(*) FROM check_results WHERE ok = 0").await,
        1
    );
    let public = app
        .get("/api/ui/public/payments/status", None)
        .await
        .text()
        .await
        .unwrap();
    assert!(!public.contains("t0ken") && !public.contains("Authorization"));
}

/// A form that signs in with `kind`, on top of a plain GET.
fn signed(url: &str, kind: &str, secret: i64, extra: Value) -> Value {
    let mut f = form(url, "GET", "", "");
    f["auth_kind"] = kind.into();
    f["auth_secret_id"] = secret.into();
    for (k, v) in extra.as_object().unwrap() {
        f[k] = v.clone();
    }
    f
}

#[tokio::test]
async fn a_monitor_signs_in_with_a_project_secret_and_never_shows_it() {
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    app.project(&tl, "Payments", "payments").await;
    app.project(&tl, "Billing", "billing").await;
    let secret = |slug: &str| format!("/api/ui/projects/{slug}/secrets");
    let value = json!({ "name": "API_TOKEN", "value": "s3cret-v4lue" });
    let id = json_of(
        app.api_post(&secret("payments"), value.clone(), Some(&tl))
            .await,
    )
    .await["data"]["id"]
        .as_i64()
        .unwrap();
    let foreign =
        json_of(app.api_post(&secret("billing"), value, Some(&tl)).await).await["data"]["id"]
            .as_i64()
            .unwrap();
    let path = "/api/ui/projects/payments/monitors";
    let (url, seen) = recorder().await;

    let refused = async |body: Value| {
        let res = app.api_post(path, body, Some(&tl)).await;
        assert_eq!(res.status(), 422);
        json_of(res).await["error"]["fields"].clone()
    };
    let other = refused(signed(&url, "bearer", foreign, json!({}))).await;
    assert!(other["auth_secret_id"].is_string(), "{other}");
    let twice = refused(signed(
        &url,
        "bearer",
        id,
        json!({ "headers": "Authorization: Bearer plain" }),
    ))
    .await;
    assert!(twice["headers"].is_string(), "{twice}");
    let no_user = refused(signed(&url, "basic", id, json!({}))).await;
    assert!(no_user["auth_user"].is_string(), "{no_user}");

    let res = app
        .api_post(path, signed(&url, "bearer", id, json!({})), Some(&tl))
        .await;
    assert_eq!(res.status(), 201);
    checks(&app, 1).await;
    assert_eq!(seen.lock().unwrap()[0].1, "Bearer s3cret-v4lue");
    let detail = app
        .api_get(&format!("{path}/1"), Some(&tl))
        .await
        .text()
        .await
        .unwrap();
    assert!(!detail.contains("s3cret"), "{detail}");
    let detail: Value = serde_json::from_str(&detail).unwrap();
    assert_eq!(detail["data"]["monitor"]["auth_secret_name"], "API_TOKEN");

    let in_use = app
        .api_send(
            reqwest::Method::DELETE,
            &format!("{}/{id}", secret("payments")),
            json!({}),
            Some(&tl),
        )
        .await;
    assert_eq!(in_use.status(), 422, "a secret in use stays");

    let edit = async |body: Value| {
        let res = app
            .api_send(reqwest::Method::PUT, &format!("{path}/1"), body, Some(&tl))
            .await;
        assert_eq!(res.status(), 204);
    };
    edit(signed(
        &url,
        "header",
        id,
        json!({ "auth_header": "X-Api-Key" }),
    ))
    .await;
    checks(&app, 2).await;
    edit(signed(&url, "basic", id, json!({ "auth_user": "svc" }))).await;
    checks(&app, 3).await;
    let got = seen.lock().unwrap().clone();
    assert_eq!((got[1].1.as_str(), got[1].3.as_str()), ("", "s3cret-v4lue"));
    assert_eq!(got[2].1, "Basic c3ZjOnMzY3JldC12NGx1ZQ==");
}
