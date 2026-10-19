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

/// (method, Authorization header, body) of each request the server got.
type Seen = Arc<Mutex<Vec<(String, String, String)>>>;

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
                        let auth = h
                            .get("authorization")
                            .and_then(|v| v.to_str().ok())
                            .unwrap_or("")
                            .to_string();
                        s.lock().unwrap().push((
                            method.to_string(),
                            auth,
                            String::from_utf8_lossy(&body).to_string(),
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
        ("POST".into(), "Bearer t0ken".into(), r#"{"ping":1}"#.into())
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
