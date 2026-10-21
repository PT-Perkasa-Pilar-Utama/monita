//! v1.1 Q24 (#7): a monitor pings a host with one ICMP echo through an unprivileged socket.

mod common;

use common::{TestApp, json_of};
use monita::{checks, platform::db::query_i64};
use serde_json::{Value, json};

const MONITORS: &str = "/api/ui/projects/ops/monitors";

fn form(url: &str) -> Value {
    json!({ "name": "Gateway", "kind": "ping", "url": url, "interval_s": "60", "timeout_s": "2",
            "expected_status": "2xx", "severity": "warning", "method": "POST", "body": "x",
            "cert_warn_days": "" })
}

async fn setup(app: &TestApp) -> String {
    let tl = app.tl("rina@x.co").await;
    app.project(&tl, "Ops", "ops").await;
    tl
}

#[tokio::test]
async fn a_ping_monitor_takes_a_host_and_keeps_no_http_fields() {
    let app = TestApp::spawn("test").await;
    let tl = setup(&app).await;
    let res = app
        .api_post(MONITORS, form("db.internal:5432"), Some(&tl))
        .await;
    assert_eq!(res.status(), 422);
    let body = json_of(res).await;
    assert!(
        body["error"]["fields"]["url"]
            .as_str()
            .unwrap()
            .contains("host name or an address")
    );

    let res = app.api_post(MONITORS, form("127.0.0.1"), Some(&tl)).await;
    assert_eq!(res.status(), 201);
    let detail = json_of(app.api_get(&format!("{MONITORS}/1"), Some(&tl)).await).await;
    let m = &detail["data"]["monitor"];
    assert_eq!(m["kind"], "ping");
    assert_eq!(m["method"], "GET", "HTTP fields go back to their defaults");
    assert!(m["body"].is_null());
}

#[tokio::test]
async fn pinging_this_machine_passes_or_names_the_sysctl_that_allows_it() {
    let app = TestApp::spawn("test").await;
    let tl = setup(&app).await;
    assert_eq!(
        app.api_post(MONITORS, form("127.0.0.1"), Some(&tl))
            .await
            .status(),
        201
    );
    let m = checks::find_monitor(&app.state.db, 1, 1)
        .await
        .unwrap()
        .unwrap();
    let r = checks::run_check(&app.state.db, &app.state.http, &m)
        .await
        .unwrap();
    if let Some(error) = &r.error {
        // CI runners differ: some refuse unprivileged ICMP sockets.
        assert!(error.contains("net.ipv4.ping_group_range"), "{error}");
        assert!(!r.ok);
        return;
    }
    assert!(r.ok);
    assert!(r.latency_ms < 2_000);
}

#[tokio::test]
async fn the_rebuilt_monitors_table_keeps_its_rows_and_its_id_counter() {
    let app = TestApp::spawn("test").await;
    let tl = setup(&app).await;
    for host in ["10.0.0.1", "10.0.0.2"] {
        assert_eq!(
            app.api_post(MONITORS, form(host), Some(&tl)).await.status(),
            201
        );
    }
    let c = app.state.db.read().await.unwrap();
    let n = query_i64(&c, "SELECT count(*) FROM monitors WHERE kind = 'ping'", ())
        .await
        .unwrap();
    assert_eq!(n, Some(2));
    // AUTOINCREMENT after the rebuild: a deleted id is never handed out again.
    drop(c);
    let del = app
        .api_send(
            reqwest::Method::DELETE,
            &format!("{MONITORS}/2"),
            json!({}),
            Some(&tl),
        )
        .await;
    assert!(del.status().is_success());
    assert_eq!(
        app.api_post(MONITORS, form("10.0.0.3"), Some(&tl))
            .await
            .status(),
        201
    );
    let c = app.state.db.read().await.unwrap();
    assert_eq!(
        query_i64(&c, "SELECT max(id) FROM monitors", ())
            .await
            .unwrap(),
        Some(3)
    );
}
