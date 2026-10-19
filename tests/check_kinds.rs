//! v1.1 Q16 (#9): TCP and DNS monitors, through the form, the scheduler and the incident engine.

mod common;

use std::time::Duration;

use common::{TestApp, json_of};
use monita::platform::db::query_i64;
use serde_json::{Value, json};

async fn count(app: &TestApp, sql: &str) -> i64 {
    query_i64(&app.state.db.read().await.unwrap(), sql, ())
        .await
        .unwrap()
        .unwrap_or(0)
}

/// Makes every monitor due now, then waits until each has `n` results: the test stands in for
/// the clock.
async fn check_all(app: &TestApp, n: i64) {
    app.state
        .db
        .write(async |c| {
            c.execute("UPDATE monitors SET next_check_at = 0", ())
                .await?;
            Ok(())
        })
        .await
        .unwrap();
    let monitors = count(app, "SELECT count(*) FROM monitors").await;
    for _ in 0..100 {
        if count(app, "SELECT count(*) FROM check_results").await >= monitors * n {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("the checks did not finish");
}

async fn create(app: &TestApp, tl: &str, form: Value) -> reqwest::Response {
    app.api_post("/api/ui/projects/payments/monitors", form, Some(tl))
        .await
}

/// The newest check of monitor `id`, as its page shows it.
async fn last_check(app: &TestApp, tl: &str, id: i64) -> Value {
    let detail = json_of(
        app.api_get(
            &format!("/api/ui/projects/payments/monitors/{id}"),
            Some(tl),
        )
        .await,
    )
    .await;
    detail["data"]["checks"][0].clone()
}

fn form(kind: &str, name: &str, target: &str) -> Value {
    json!({ "kind": kind, "name": name, "url": target, "interval_s": "60", "timeout_s": "5",
            "expected_status": "2xx", "severity": "warning" })
}

async fn setup(app: &TestApp) -> String {
    let tl = app.tl("rina@x.co").await;
    app.project(&tl, "Payments", "payments").await;
    tl
}

#[tokio::test]
async fn a_tcp_check_passes_on_an_open_port_and_fails_on_a_closed_one() {
    let app = TestApp::spawn("test").await;
    let tl = setup(&app).await;
    let open = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let closed = {
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        l.local_addr().unwrap()
    };
    let open_at = open.local_addr().unwrap().to_string();
    assert_eq!(
        create(&app, &tl, form("tcp", "Database", &open_at))
            .await
            .status(),
        201
    );
    let closed_at = closed.to_string();
    assert_eq!(
        create(&app, &tl, form("tcp", "Queue", &closed_at))
            .await
            .status(),
        201
    );
    check_all(&app, 1).await;

    let up = last_check(&app, &tl, 1).await;
    assert_eq!(up["ok"], true, "{up}");
    assert!(
        up["status_code"].is_null(),
        "a TCP check has no status code"
    );
    let down = last_check(&app, &tl, 2).await;
    assert_eq!(down["ok"], false);
    assert!(
        down["error"]
            .as_str()
            .unwrap()
            .starts_with("Connection failed"),
        "{down}"
    );
}

#[tokio::test]
async fn a_dns_check_compares_the_answers_with_the_expected_address() {
    let app = TestApp::spawn("test").await;
    let tl = setup(&app).await;
    let mut any = form("dns", "Resolves", "localhost");
    any["expected_address"] = json!("");
    assert_eq!(create(&app, &tl, any).await.status(), 201);
    let mut right = form("dns", "Right address", "localhost");
    right["expected_address"] = json!("127.0.0.1");
    assert_eq!(create(&app, &tl, right).await.status(), 201);
    let mut wrong = form("dns", "Wrong address", "localhost");
    wrong["expected_address"] = json!("203.0.113.10");
    assert_eq!(create(&app, &tl, wrong).await.status(), 201);
    let missing = form("dns", "No such name", "monita-test.invalid");
    assert_eq!(create(&app, &tl, missing).await.status(), 201);
    check_all(&app, 1).await;

    assert_eq!(last_check(&app, &tl, 1).await["ok"], true);
    assert_eq!(last_check(&app, &tl, 2).await["ok"], true);
    let wrong = last_check(&app, &tl, 3).await;
    assert_eq!(wrong["ok"], false);
    let error = wrong["error"].as_str().unwrap();
    assert!(
        error.starts_with("Resolved to ") && error.ends_with("expected 203.0.113.10"),
        "{error}"
    );
    let missing = last_check(&app, &tl, 4).await;
    assert_eq!(missing["ok"], false);
    assert!(
        missing["error"]
            .as_str()
            .unwrap()
            .starts_with("Lookup failed"),
        "{missing}"
    );
}

#[tokio::test]
async fn a_failing_tcp_check_opens_an_incident_like_an_http_one() {
    let app = TestApp::spawn("test").await;
    let tl = setup(&app).await;
    let closed = {
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        l.local_addr().unwrap()
    };
    let mut f = form("tcp", "Queue", &closed.to_string());
    f["fails_to_open"] = json!("1");
    assert_eq!(create(&app, &tl, f).await.status(), 201);
    check_all(&app, 1).await;
    for _ in 0..50 {
        if count(
            &app,
            "SELECT count(*) FROM incidents WHERE resolved_at IS NULL",
        )
        .await
            == 1
        {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("no incident opened");
}

#[tokio::test]
async fn the_form_checks_each_kind_and_keeps_no_http_fields_on_the_others() {
    let app = TestApp::spawn("test").await;
    let tl = setup(&app).await;
    let field = async |f: Value| {
        let res = create(&app, &tl, f).await;
        assert_eq!(res.status(), 422);
        let err = json_of(res).await;
        err["error"]["fields"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect::<Vec<_>>()
    };
    assert_eq!(field(form("tcp", "No port", "db.internal")).await, ["url"]);
    assert_eq!(
        field(form("dns", "Has a scheme", "https://x.co")).await,
        ["url"]
    );
    let mut bad = form("dns", "Bad address", "x.co");
    bad["expected_address"] = json!("not-an-ip");
    assert_eq!(field(bad).await, ["expected_address"]);
    assert_eq!(field(form("ping", "Unknown", "x.co")).await, ["kind"]);

    let mut tcp = form("tcp", "Database", "[::1]:5432");
    tcp["headers"] = json!("X-Env: prod");
    tcp["keyword"] = json!("ok");
    tcp["expected_address"] = json!("127.0.0.1");
    assert_eq!(create(&app, &tl, tcp).await.status(), 201);
    let detail = json_of(
        app.api_get("/api/ui/projects/payments/monitors/1", Some(&tl))
            .await,
    )
    .await;
    let m = &detail["data"]["monitor"];
    assert_eq!(m["kind"], "tcp");
    assert_eq!(m["url"], "[::1]:5432");
    assert_eq!(m["headers"], "", "HTTP fields are not kept");
    assert!(m["keyword"].is_null());
    assert!(
        m["expected_address"].is_null(),
        "only a DNS check keeps an address"
    );
}
