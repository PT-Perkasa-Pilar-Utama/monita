//! v1.1 Q26 (#6), technical-specs 14: locations, the probe API, the verdict across places, and
//! the `monita probe` command against a running server.

mod common;

use std::time::Duration;

use common::{BASE_URL, PASSWORD, TestApp, json_of};
use monita::{
    checks::{self, CheckResult},
    platform::{db::query_i64, now},
};
use reqwest::Method;
use serde_json::{Value, json};

const ADMIN: &str = "/api/ui/admin/locations";
const MONITORS: &str = "/api/ui/projects/payments/monitors";

async fn count(app: &TestApp, sql: &str) -> i64 {
    query_i64(&app.state.db.read().await.unwrap(), sql, ())
        .await
        .unwrap()
        .unwrap_or(0)
}

async fn sql(app: &TestApp, sql: &str) {
    app.state
        .db
        .write(async |c| {
            c.execute_batch(sql).await?;
            Ok(())
        })
        .await
        .unwrap();
}

/// A super admin, a tech lead and the project. Returns (admin, tl) cookies.
async fn setup(app: &TestApp) -> (String, String) {
    let admin = app.super_admin("admin@x.co").await;
    let tl = app.tl("rina@x.co").await;
    app.project(&tl, "Payments", "payments").await;
    (admin, tl)
}

/// Adds a location; returns (id, token).
async fn location(app: &TestApp, admin: &str, name: &str) -> (i64, String) {
    let res = app
        .api_post(ADMIN, json!({ "name": name }), Some(admin))
        .await;
    assert_eq!(res.status(), 201);
    let body = json_of(res).await;
    (
        body["data"]["id"].as_i64().unwrap(),
        body["data"]["token"].as_str().unwrap().to_string(),
    )
}

fn monitor(url: &str, locations: &[i64], quorum: &str) -> Value {
    json!({ "name": "Checkout", "url": url, "interval_s": "60", "timeout_s": "5", "expected_status": "2xx",
            "severity": "critical", "fails_to_open": "1", "location_ids": locations, "quorum": quorum })
}

async fn probe(
    app: &TestApp,
    method: Method,
    path: &str,
    token: Option<&str>,
    body: Value,
) -> reqwest::Response {
    let mut req = app.http.request(method, app.url(path)).json(&body);
    if let Some(t) = token {
        req = req.bearer_auth(t);
    }
    req.send().await.unwrap()
}

fn result(monitor_id: i64, ok: bool) -> Value {
    json!({ "monitor_id": monitor_id, "ok": ok, "status_code": if ok { 200 } else { 503 }, "latency_ms": 40,
            "error": if ok { Value::Null } else { json!("HTTP 503") } })
}

#[tokio::test]
async fn a_super_admin_adds_locations_and_the_token_shows_once() {
    let app = TestApp::spawn("test").await;
    let (admin, tl) = setup(&app).await;
    let (_, token) = location(&app, &admin, "Jakarta").await;
    assert!(token.starts_with("ml_") && token.len() == 29, "{token}");

    let list = json_of(app.api_get(ADMIN, Some(&admin)).await).await;
    let row = &list["data"]["locations"][0];
    assert_eq!(row["name"], "Jakarta");
    assert_eq!(row["token_prefix"], &token[3..7]);
    assert!(row["last_seen_at"].is_null());
    assert!(
        !list.to_string().contains(&token),
        "the list never shows the token"
    );
    assert_eq!(
        count(
            &app,
            &format!(
                "SELECT count(*) FROM locations WHERE token_hash = '{}'",
                monita::platform::token::hash(&token)
            )
        )
        .await,
        1
    );

    let res = app
        .api_post(ADMIN, json!({ "name": " Jakarta " }), Some(&admin))
        .await;
    assert_eq!(res.status(), 422, "a taken name");
    let res = app
        .api_post(ADMIN, json!({ "name": "" }), Some(&admin))
        .await;
    assert_eq!(res.status(), 422);
    for i in 1..10 {
        location(&app, &admin, &format!("Place {i}")).await;
    }
    let res = app
        .api_post(ADMIN, json!({ "name": "Eleventh" }), Some(&admin))
        .await;
    assert_eq!(res.status(), 422, "ten at most");

    assert_eq!(
        app.api_get(ADMIN, Some(&tl)).await.status(),
        403,
        "a tech lead"
    );
}

#[tokio::test]
async fn a_supervisor_reads_locations_and_changes_none() {
    let app = TestApp::spawn("test").await;
    let (admin, _) = setup(&app).await;
    let (id, _) = location(&app, &admin, "Jakarta").await;
    let link = monita::accounts::invite_tl(&app.state.db, BASE_URL, "sup@x.co")
        .await
        .unwrap();
    let sup = app
        .accept(link.strip_prefix(BASE_URL).unwrap(), PASSWORD)
        .await;
    let users = json_of(app.api_get("/api/ui/admin/users", Some(&admin)).await).await;
    let sup_id = users["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["email"] == "sup@x.co")
        .unwrap()["id"]
        .clone();
    let role = app
        .api_send(
            Method::PUT,
            &format!("/api/ui/admin/users/{sup_id}/role"),
            json!({ "role": "supervisor" }),
            Some(&admin),
        )
        .await;
    assert_eq!(role.status(), 204);

    assert_eq!(app.api_get(ADMIN, Some(&sup)).await.status(), 200);
    assert_eq!(
        app.api_post(ADMIN, json!({ "name": "Tokyo" }), Some(&sup))
            .await
            .status(),
        403
    );
    let del = app
        .api_send(
            Method::DELETE,
            &format!("{ADMIN}/{id}"),
            json!({}),
            Some(&sup),
        )
        .await;
    assert_eq!(del.status(), 403);
}

#[tokio::test]
async fn the_probe_api_needs_the_token_and_hands_out_each_due_check_once() {
    let app = TestApp::spawn("test").await;
    let (admin, tl) = setup(&app).await;
    let (loc, token) = location(&app, &admin, "Jakarta").await;
    const WORK: &str = "/api/v1/probe/work";
    assert_eq!(
        probe(&app, Method::GET, WORK, None, json!({}))
            .await
            .status(),
        401
    );
    assert_eq!(
        probe(&app, Method::GET, WORK, Some("ml_wrong"), json!({}))
            .await
            .status(),
        401
    );
    assert_eq!(
        probe(
            &app,
            Method::GET,
            &format!("{WORK}?free=0"),
            Some(&token),
            json!({})
        )
        .await
        .status(),
        422
    );
    let empty = json_of(probe(&app, Method::GET, WORK, Some(&token), json!({})).await).await;
    assert_eq!(empty["data"]["jobs"], json!([]));
    assert_eq!(
        count(
            &app,
            "SELECT count(*) FROM locations WHERE last_seen_at IS NOT NULL"
        )
        .await,
        1
    );

    // Signing in: the job carries the secret's value, which only a location token can fetch.
    let secret = json_of(
        app.api_post(
            "/api/ui/projects/payments/secrets",
            json!({ "name": "TOKEN", "value": "s3cret" }),
            Some(&tl),
        )
        .await,
    )
    .await["data"]["id"]
        .clone();
    let mut form = monitor("https://pay.example.com/health", &[loc], "1");
    form["auth_kind"] = json!("bearer");
    form["auth_secret_id"] = secret;
    assert_eq!(app.api_post(MONITORS, form, Some(&tl)).await.status(), 201);
    sql(&app, "UPDATE monitor_locations SET next_check_at = 0; UPDATE monitors SET next_check_at = 9999999999").await;

    let work = json_of(probe(&app, Method::GET, WORK, Some(&token), json!({})).await).await;
    let jobs = work["data"]["jobs"].as_array().unwrap();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0]["target"], "https://pay.example.com/health");
    assert_eq!(jobs[0]["auth"]["secret"], "s3cret");
    let again = json_of(probe(&app, Method::GET, WORK, Some(&token), json!({})).await).await;
    assert_eq!(
        again["data"]["jobs"],
        json!([]),
        "claimed one interval ahead"
    );
}

/// Makes every assigned check due, and claims them as the location's probe would.
async fn claim(app: &TestApp, token: &str) -> usize {
    sql(app, "UPDATE monitor_locations SET next_check_at = 0").await;
    let work = json_of(
        probe(
            app,
            Method::GET,
            "/api/v1/probe/work",
            Some(token),
            json!({}),
        )
        .await,
    )
    .await;
    work["data"]["jobs"].as_array().unwrap().len()
}

#[tokio::test]
async fn a_check_fails_only_when_the_quorum_of_places_agrees() {
    let app = TestApp::spawn("test").await;
    let (admin, tl) = setup(&app).await;
    let (loc, token) = location(&app, &admin, "Jakarta").await;
    let res = app
        .api_post(
            MONITORS,
            monitor("https://pay.example.com", &[loc], "3"),
            Some(&tl),
        )
        .await;
    assert_eq!(res.status(), 422, "two places cannot need three");
    let res = app
        .api_post(
            MONITORS,
            monitor("https://pay.example.com", &[999], "1"),
            Some(&tl),
        )
        .await;
    assert_eq!(res.status(), 422, "an unknown location");
    let res = app
        .api_post(
            MONITORS,
            monitor("https://pay.example.com", &[loc], "2"),
            Some(&tl),
        )
        .await;
    assert_eq!(res.status(), 201);
    sql(&app, "UPDATE monitors SET next_check_at = 9999999999").await;
    const RESULTS: &str = "/api/v1/probe/results";
    const OPEN: &str = "SELECT count(*) FROM incidents WHERE resolved_at IS NULL";
    let local = async |ok: bool| {
        let m = checks::find_monitor(&app.state.db, 1, 1)
            .await
            .unwrap()
            .unwrap();
        let r = CheckResult {
            checked_at: now(),
            ok,
            status_code: None,
            latency_ms: 5000,
            error: (!ok).then(|| "Timeout after 5000 ms".into()),
            cert_expires_at: None,
        };
        checks::record_check(&app.state.db, BASE_URL, &m, &r)
            .await
            .unwrap();
    };

    // A result that answers no claim is skipped.
    let unclaimed = probe(
        &app,
        Method::POST,
        RESULTS,
        Some(&token),
        json!({ "results": [result(1, false)] }),
    )
    .await;
    assert_eq!(
        json_of(unclaimed).await["data"],
        json!({ "stored": 0, "skipped": 1 })
    );

    assert_eq!(claim(&app, &token).await, 1);
    let ok = probe(
        &app,
        Method::POST,
        RESULTS,
        Some(&token),
        json!({ "results": [result(1, true), result(1, true)] }),
    )
    .await;
    assert_eq!(
        json_of(ok).await["data"],
        json!({ "stored": 1, "skipped": 1 }),
        "one result per claim"
    );
    local(false).await;
    assert_eq!(
        count(&app, OPEN).await,
        0,
        "one of two places is short of a quorum of 2"
    );

    assert_eq!(claim(&app, &token).await, 1);
    let res = probe(
        &app,
        Method::POST,
        RESULTS,
        Some(&token),
        json!({ "results": [result(1, false), result(77, false)] }),
    )
    .await;
    assert_eq!(
        json_of(res).await["data"],
        json!({ "stored": 1, "skipped": 1 })
    );
    assert_eq!(
        count(&app, OPEN).await,
        0,
        "a location's result feeds no verdict by itself"
    );
    local(false).await;
    assert_eq!(
        count(&app, OPEN).await,
        1,
        "this server's next check sees both places fail"
    );
    let cause: String = {
        let c = app.state.db.read().await.unwrap();
        let mut rows = c
            .query("SELECT cause FROM incidents WHERE resolved_at IS NULL", ())
            .await
            .unwrap();
        rows.next().await.unwrap().unwrap().get(0).unwrap()
    };
    assert!(
        cause.starts_with(
            "2 of 2 places failed: This server: Timeout after 5000 ms; Jakarta: HTTP 503"
        ),
        "{cause}"
    );

    let detail = json_of(app.api_get(&format!("{MONITORS}/1"), Some(&tl)).await).await;
    assert_eq!(detail["data"]["monitor"]["quorum"], 2);
    assert_eq!(detail["data"]["location_ids"], json!([loc]));
    let places = detail["data"]["places"].as_array().unwrap();
    assert_eq!(places[0]["name"], "This server");
    assert_eq!(places[1]["name"], "Jakarta");
    let checks_list = detail["data"]["checks"].as_array().unwrap();
    assert!(checks_list.iter().any(|c| c["place"] == "Jakarta"));
    assert!(checks_list.iter().any(|c| c["place"].is_null()));
}

#[tokio::test]
async fn deleting_a_location_drops_its_results_and_lowers_the_quorum() {
    let app = TestApp::spawn("test").await;
    let (admin, tl) = setup(&app).await;
    let (loc, token) = location(&app, &admin, "Jakarta").await;
    assert_eq!(
        app.api_post(
            MONITORS,
            monitor("https://pay.example.com", &[loc], "2"),
            Some(&tl)
        )
        .await
        .status(),
        201
    );
    assert_eq!(claim(&app, &token).await, 1);
    probe(
        &app,
        Method::POST,
        "/api/v1/probe/results",
        Some(&token),
        json!({ "results": [result(1, true)] }),
    )
    .await;
    assert_eq!(
        count(
            &app,
            "SELECT count(*) FROM check_results WHERE location_id IS NOT NULL"
        )
        .await,
        1
    );

    let del = app
        .api_send(
            Method::DELETE,
            &format!("{ADMIN}/{loc}"),
            json!({}),
            Some(&admin),
        )
        .await;
    assert_eq!(del.status(), 204);
    assert_eq!(
        count(
            &app,
            "SELECT count(*) FROM check_results WHERE location_id IS NOT NULL"
        )
        .await,
        0
    );
    assert_eq!(
        count(&app, "SELECT count(*) FROM monitor_locations").await,
        0
    );
    assert_eq!(
        count(&app, "SELECT quorum FROM monitors WHERE id = 1").await,
        1
    );
    let gone = probe(
        &app,
        Method::GET,
        "/api/v1/probe/work",
        Some(&token),
        json!({}),
    )
    .await;
    assert_eq!(gone.status(), 401, "a deleted location's token is dead");
}

#[tokio::test]
async fn the_probe_command_runs_a_location_check_and_reports_it() {
    let app = TestApp::spawn("test").await;
    let (admin, tl) = setup(&app).await;
    let (loc, token) = location(&app, &admin, "Jakarta").await;
    let target = format!("{}/healthz", app.base);
    assert_eq!(
        app.api_post(MONITORS, monitor(&target, &[loc], "1"), Some(&tl))
            .await
            .status(),
        201
    );
    sql(&app, "UPDATE monitor_locations SET next_check_at = 0; UPDATE monitors SET next_check_at = 9999999999").await;

    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_monita"))
        .args(["probe", "--server", &app.base, "--concurrency", "2"])
        .env("MONITA_PROBE_TOKEN", &token)
        .env("MONITA_LOG", "warn")
        .spawn()
        .unwrap();
    let mut stored = 0;
    for _ in 0..50 {
        stored = count(
            &app,
            "SELECT count(*) FROM check_results WHERE location_id IS NOT NULL AND ok = 1",
        )
        .await;
        if stored > 0 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    child.kill().unwrap();
    child.wait().unwrap();
    assert_eq!(
        stored, 1,
        "the probe checked /healthz from its location and reported a pass"
    );
}

#[tokio::test]
async fn a_place_newest_result_is_the_latest_by_time_not_the_last_stored() {
    let app = TestApp::spawn("test").await;
    let (admin, tl) = setup(&app).await;
    let (loc, _) = location(&app, &admin, "Jakarta").await;
    assert_eq!(
        app.api_post(
            MONITORS,
            monitor("https://pay.example.com", &[loc], "1"),
            Some(&tl)
        )
        .await
        .status(),
        201
    );
    sql(&app, "UPDATE monitors SET next_check_at = 9999999999").await;
    let m = checks::find_monitor(&app.state.db, 1, 1)
        .await
        .unwrap()
        .unwrap();
    let at = |checked_at, ok| CheckResult {
        checked_at,
        ok,
        status_code: Some(if ok { 200 } else { 503 }),
        latency_ms: 10,
        error: (!ok).then(|| "HTTP 503".into()),
        cert_expires_at: None,
    };
    let t = now();
    checks::record_check(&app.state.db, BASE_URL, &m, &at(t, true))
        .await
        .unwrap();
    // Stored after the pass, but checked a minute before it: the pass is still the newest.
    checks::record_check(&app.state.db, BASE_URL, &m, &at(t - 60, false))
        .await
        .unwrap();
    assert_eq!(count(&app, "SELECT count(*) FROM incidents").await, 0);
    let detail = json_of(app.api_get(&format!("{MONITORS}/1"), Some(&tl)).await).await;
    assert_eq!(detail["data"]["places"][0]["last"][1], true);
}

#[tokio::test]
async fn a_slow_probe_keeps_both_results_and_a_stale_claim_takes_none() {
    let app = TestApp::spawn("test").await;
    let (admin, tl) = setup(&app).await;
    let (loc, token) = location(&app, &admin, "Jakarta").await;
    let res = app
        .api_post(
            MONITORS,
            monitor("https://pay.example.com", &[loc], "1"),
            Some(&tl),
        )
        .await;
    assert_eq!(res.status(), 201);
    sql(&app, "UPDATE monitors SET next_check_at = 9999999999").await;
    const RESULTS: &str = "/api/v1/probe/results";
    // Handed the same check twice before it reports: both results count, a third does not.
    assert_eq!(claim(&app, &token).await, 1);
    assert_eq!(claim(&app, &token).await, 1);
    let three = json!({ "results": [result(1, true), result(1, true), result(1, true)] });
    let res = probe(&app, Method::POST, RESULTS, Some(&token), three).await;
    assert_eq!(
        json_of(res).await["data"],
        json!({ "stored": 2, "skipped": 1 })
    );

    // A claim older than two intervals, the timeout and 30 s (155 s here) is stale.
    assert_eq!(claim(&app, &token).await, 1);
    sql(
        &app,
        &format!("UPDATE monitor_locations SET claimed_at = {}", now() - 156),
    )
    .await;
    let late = probe(
        &app,
        Method::POST,
        RESULTS,
        Some(&token),
        json!({ "results": [result(1, false)] }),
    )
    .await;
    assert_eq!(
        json_of(late).await["data"],
        json!({ "stored": 0, "skipped": 1 })
    );
}

#[tokio::test]
async fn claims_a_probe_never_answered_do_not_credit_a_later_result() {
    let app = TestApp::spawn("test").await;
    let (admin, tl) = setup(&app).await;
    let (loc, token) = location(&app, &admin, "Jakarta").await;
    let res = app
        .api_post(
            MONITORS,
            monitor("https://pay.example.com", &[loc], "1"),
            Some(&tl),
        )
        .await;
    assert_eq!(res.status(), 201);
    sql(&app, "UPDATE monitors SET next_check_at = 9999999999").await;
    assert_eq!(claim(&app, &token).await, 1);
    assert_eq!(claim(&app, &token).await, 1);
    // The probe went away; both claims are now older than 155 s (60 s interval, 5 s timeout).
    sql(
        &app,
        &format!("UPDATE monitor_locations SET claimed_at = {}", now() - 300),
    )
    .await;
    assert_eq!(claim(&app, &token).await, 1);
    let two = json!({ "results": [result(1, true), result(1, true)] });
    let res = probe(
        &app,
        Method::POST,
        "/api/v1/probe/results",
        Some(&token),
        two,
    )
    .await;
    assert_eq!(
        json_of(res).await["data"],
        json!({ "stored": 1, "skipped": 1 })
    );
}
