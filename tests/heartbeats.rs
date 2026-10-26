//! US-12 to US-14. The clock is moved through deadline_at.

mod common;

use std::time::Duration;

use common::{BASE_URL, RESET_TOKEN, TestApp, json_of};
use monita::{
    checks::heartbeat::{self, HeartbeatInput},
    platform::db::query_i64,
};
use serde_json::json;

async fn count(app: &TestApp, sql: &str) -> i64 {
    query_i64(&app.state.db.read().await.unwrap(), sql, ())
        .await
        .unwrap()
        .unwrap_or(0)
}

async fn setup(app: &TestApp) -> (String, String) {
    let tl = app.tl("rina@x.co").await;
    app.project(&tl, "Payments", "payments").await;
    let id = heartbeat::create(
        &app.state.db,
        1,
        &HeartbeatInput {
            name: "nightly".into(),
            ..HeartbeatInput::default()
        },
    )
    .await
    .unwrap()
    .unwrap();
    let token = heartbeat::find(&app.state.db, 1, id)
        .await
        .unwrap()
        .unwrap()
        .token;
    (tl, token)
}

/// Pings from a job have no browser origin and no cookie.
async fn ping(app: &TestApp, method: reqwest::Method, path: &str) -> reqwest::Response {
    app.http
        .request(method, app.url(path))
        .send()
        .await
        .unwrap()
}

async fn state_label(app: &TestApp, tl: &str) -> String {
    let d = json_of(
        app.api_get("/api/ui/projects/payments/heartbeats/1", Some(tl))
            .await,
    )
    .await;
    d["data"]["heartbeat"]["state"]
        .as_str()
        .unwrap()
        .to_string()
}

#[tokio::test]
async fn tl_creates_a_heartbeat_and_sees_its_ping_url() {
    // AC-12.01, AC-12.02
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    app.project(&tl, "Payments", "payments").await;
    let o = json_of(app.api_get("/api/ui/check-options", Some(&tl)).await).await;
    assert_eq!(
        o["data"]["heartbeat"],
        json!({ "period_s": 86_400, "grace_s": 1_800, "severity": "warning" })
    );
    let res = app
        .api_post(
            "/api/ui/projects/payments/heartbeats",
            json!({
                "name": "nightly-settlement",
                "period_s": "3600",
                "grace_s": "300",
                "severity": "critical",
            }),
            Some(&tl),
        )
        .await;
    assert_eq!(res.status(), 201);
    let id = json_of(res).await["data"]["id"].as_i64().unwrap();
    let d = json_of(
        app.api_get(
            &format!("/api/ui/projects/payments/heartbeats/{id}"),
            Some(&tl),
        )
        .await,
    )
    .await;
    let token = heartbeat::find(&app.state.db, 1, id)
        .await
        .unwrap()
        .unwrap()
        .token;
    assert_eq!(token.len(), 26);
    assert_eq!(d["data"]["ping_url"], format!("{BASE_URL}/ping/{token}"));
    assert_eq!(d["data"]["heartbeat"]["state"], "unknown", "waiting");
    assert_eq!(d["data"]["heartbeat"]["severity"], "critical");
}

#[tokio::test]
async fn tl_edits_and_deletes_a_heartbeat() {
    let app = TestApp::spawn("test").await;
    let (tl, token) = setup(&app).await;
    let path = "/api/ui/projects/payments/heartbeats/1";
    let form = json!({ "name": "nightly v2", "period_s": "3600", "grace_s": "300", "severity": "warning" });
    let res = app
        .api_send(reqwest::Method::PUT, path, form, Some(&tl))
        .await;
    assert_eq!(res.status(), 204);
    let d = json_of(app.api_get(path, Some(&tl)).await).await;
    assert_eq!(d["data"]["heartbeat"]["name"], "nightly v2");
    assert_eq!(d["data"]["heartbeat"]["period_s"], 3600);

    let res = app
        .api_send(reqwest::Method::DELETE, path, json!({}), Some(&tl))
        .await;
    assert_eq!(res.status(), 204);
    assert_eq!(app.api_get(path, Some(&tl)).await.status(), 404);
    let res = ping(&app, reqwest::Method::GET, &format!("/ping/{token}")).await;
    assert_eq!(res.status(), 404, "a deleted heartbeat's URL is gone");
}

#[tokio::test]
async fn get_head_and_post_all_count_as_a_ping() {
    // AC-13.01
    let app = TestApp::spawn("test").await;
    let (tl, token) = setup(&app).await;
    let path = format!("/ping/{token}");
    for (i, method) in [
        reqwest::Method::GET,
        reqwest::Method::HEAD,
        reqwest::Method::POST,
    ]
    .into_iter()
    .enumerate()
    {
        if i > 0 {
            tokio::time::sleep(Duration::from_millis(1_050)).await;
        }
        let res = ping(&app, method, &path).await;
        assert_eq!(res.status(), 200);
    }
    assert_eq!(
        count(&app, "SELECT count(*) FROM pings WHERE kind = 'ok'").await,
        3
    );
    assert_eq!(state_label(&app, &tl).await, "up");
    let deadline = count(&app, "SELECT deadline_at FROM heartbeats").await;
    let now = monita::platform::now();
    assert!(
        deadline > now + 86_400,
        "deadline moved to ping + period + grace"
    );
}

#[tokio::test]
async fn a_second_ping_within_a_second_gets_429_and_the_heartbeat_stays_up() {
    // AC-13.02
    let app = TestApp::spawn("test").await;
    let (tl, token) = setup(&app).await;
    let path = format!("/ping/{token}");
    // Three pings inside one second: whichever second boundary they straddle, one is refused.
    let mut statuses = Vec::new();
    let mut retry_after = None;
    for _ in 0..3 {
        let res = ping(&app, reqwest::Method::GET, &path).await;
        if res.status() == 429 {
            retry_after = Some(res.headers()["retry-after"].to_str().unwrap().to_string());
        }
        statuses.push(res.status().as_u16());
    }
    assert_eq!(statuses[0], 200);
    assert!(statuses.contains(&429), "{statuses:?}");
    assert_eq!(retry_after.as_deref(), Some("1"));
    assert_eq!(state_label(&app, &tl).await, "up");
}

#[tokio::test]
async fn a_fail_ping_marks_the_heartbeat_down_at_once() {
    // AC-14.01
    let app = TestApp::spawn("test").await;
    let (tl, token) = setup(&app).await;
    let res = ping(&app, reqwest::Method::POST, &format!("/ping/{token}/fail")).await;
    assert_eq!(res.status(), 200);
    assert_eq!(state_label(&app, &tl).await, "down");
    assert_eq!(
        count(&app, "SELECT count(*) FROM pings WHERE kind = 'fail'").await,
        1
    );
}

#[tokio::test]
async fn the_list_shows_the_newest_ping_even_when_it_failed() {
    let app = TestApp::spawn("test").await;
    let (tl, token) = setup(&app).await;
    let res = app
        .http
        .post(app.url(&format!("/ping/{token}/fail")))
        .body("disk full")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let list = json_of(
        app.api_get("/api/ui/projects/payments/heartbeats", Some(&tl))
            .await,
    )
    .await;
    let hb = &list["data"]["heartbeats"][0];
    assert!(hb["last_ping_at"].is_null(), "no good ping yet");
    assert_eq!(hb["last"]["ok"], false);
    assert_eq!(hb["last"]["cause"], "disk full");
    let detail = json_of(
        app.api_get("/api/ui/projects/payments/heartbeats/1", Some(&tl))
            .await,
    )
    .await;
    let stats = &detail["data"]["stats"];
    assert_eq!(stats[0]["label"], "Down for", "a fail ping counts as heard");
    assert_eq!(stats[1]["label"], "Next ping due");
    assert!(
        stats[1]["value"].as_str().unwrap().starts_with("in "),
        "{stats}"
    );
    assert_eq!(stats[2]["sub"], "the job reported a failure");
    assert_eq!(detail["data"]["pings"][0]["cause"], "disk full");

    let res = ping(&app, reqwest::Method::GET, &format!("/ping/{token}")).await;
    assert_eq!(res.status(), 200);
    let list = json_of(
        app.api_get("/api/ui/projects/payments/heartbeats", Some(&tl))
            .await,
    )
    .await;
    assert_eq!(list["data"]["heartbeats"][0]["last"]["ok"], true);
}

#[tokio::test]
async fn a_fail_right_after_a_success_ping_is_still_heard() {
    let app = TestApp::spawn("test").await;
    let (tl, token) = setup(&app).await;
    let ok = ping(&app, reqwest::Method::GET, &format!("/ping/{token}")).await;
    assert_eq!(ok.status(), 200);
    let fail = ping(&app, reqwest::Method::POST, &format!("/ping/{token}/fail")).await;
    assert_eq!(fail.status(), 200);
    assert_eq!(state_label(&app, &tl).await, "down");
    assert_eq!(
        count(
            &app,
            "SELECT count(*) FROM incidents WHERE resolved_at IS NULL"
        )
        .await,
        1
    );
}

#[tokio::test]
async fn a_long_fail_body_still_opens_the_incident_with_its_first_kb() {
    let app = TestApp::spawn("test").await;
    let (tl, token) = setup(&app).await;
    let log = format!("disk full on /var\n{}", "x".repeat(3000));
    let res = app
        .http
        .post(app.url(&format!("/ping/{token}/fail")))
        .body(log)
        .send()
        .await
        .unwrap();
    assert_eq!(
        res.status(),
        200,
        "the failure is never dropped for its size"
    );
    assert_eq!(state_label(&app, &tl).await, "down");
    assert_eq!(
        count(
            &app,
            "SELECT count(*) FROM pings WHERE kind = 'fail' AND length(cause) = 1024 AND cause LIKE 'disk full on /var%'"
        )
        .await,
        1
    );
}

#[tokio::test]
async fn an_unknown_token_is_answered_before_its_body_is_read() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let app = TestApp::spawn("test").await;
    let addr = app.base.trim_start_matches("http://").to_string();
    let mut tcp = tokio::net::TcpStream::connect(addr).await.unwrap();
    // A body announced but never sent: a server that read it first would wait for ever.
    tcp.write_all(
        b"POST /ping/nosuchtoken/fail HTTP/1.1\r\nHost: x\r\nContent-Length: 1000000\r\n\r\nfirst bytes",
    )
    .await
    .unwrap();
    let mut answer = [0u8; 12];
    tokio::time::timeout(Duration::from_secs(2), tcp.read_exact(&mut answer))
        .await
        .expect("answered without the body")
        .unwrap();
    assert_eq!(&answer, b"HTTP/1.1 404");
}

#[tokio::test]
async fn a_fail_body_of_megabytes_is_read_to_its_end_and_answered() {
    let app = TestApp::spawn("test").await;
    let (tl, token) = setup(&app).await;
    let res = app
        .http
        .post(app.url(&format!("/ping/{token}/fail")))
        // Megabytes past the kept 1 KB: an unread rest can reach the client as a reset.
        .body(format!("disk full\n{}", "x".repeat(4 * 1024 * 1024)))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200, "cut, never 413 or a reset");
    assert_eq!(state_label(&app, &tl).await, "down");
    assert_eq!(
        count(
            &app,
            "SELECT count(*) FROM pings WHERE length(cause) = 1024 AND cause LIKE 'disk full%'"
        )
        .await,
        1
    );
}

#[tokio::test]
async fn a_fail_is_stored_while_the_rest_of_its_body_still_uploads() {
    use tokio::io::AsyncWriteExt;
    let app = TestApp::spawn("test").await;
    let (_tl, token) = setup(&app).await;
    let addr = app.base.trim_start_matches("http://").to_string();
    let mut tcp = tokio::net::TcpStream::connect(addr).await.unwrap();
    let head = format!(
        "POST /ping/{token}/fail HTTP/1.1\r\nHost: x\r\nContent-Length: 4000000\r\n\r\ndisk full\n"
    );
    tcp.write_all(head.as_bytes()).await.unwrap();
    tcp.write_all(&[b'x'; 2000]).await.unwrap();
    let stored = "SELECT count(*) FROM pings WHERE kind = 'fail' AND cause LIKE 'disk full%'";
    let mut found = 0;
    // The body never ends and its deadline is 10 s, so any wait under that proves the store came
    // first. 5 s leaves room for a slow runner.
    for _ in 0..100 {
        found = count(&app, stored).await;
        if found == 1 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert_eq!(found, 1, "stored before the body ends");
    drop(tcp);
}

/// Sends `request` while the writer is held, hangs up, and returns the `pings` rows that match
/// `filter`.
async fn stored_after_hang_up(app: &TestApp, request: String, filter: &str) -> i64 {
    use tokio::io::AsyncWriteExt;
    // Hold the writer, so the client is gone while `record` waits for it.
    let db = app.state.db.clone();
    let held = tokio::spawn(async move {
        db.write(async |_| {
            tokio::time::sleep(Duration::from_millis(500)).await;
            Ok(())
        })
        .await
    });
    tokio::time::sleep(Duration::from_millis(50)).await;
    let addr = app.base.trim_start_matches("http://").to_string();
    let mut tcp = tokio::net::TcpStream::connect(addr).await.unwrap();
    tcp.write_all(request.as_bytes()).await.unwrap();
    // Gone after the handler starts. hyper never starts one for a client that closes at once.
    tokio::time::sleep(Duration::from_millis(100)).await;
    drop(tcp);
    held.await.unwrap().unwrap();
    let stored = format!("SELECT count(*) FROM pings WHERE {filter}");
    let mut found = 0;
    for _ in 0..100 {
        found = count(app, &stored).await;
        if found == 1 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    found
}

#[tokio::test]
async fn a_ping_whose_client_hangs_up_is_still_stored_on_both_routes() {
    let app = TestApp::spawn("test").await;
    let (_tl, token) = setup(&app).await;
    let fail = format!(
        "POST /ping/{token}/fail HTTP/1.1\r\nHost: x\r\nContent-Length: 9\r\n\r\ndisk full"
    );
    assert_eq!(
        stored_after_hang_up(&app, fail, "kind = 'fail' AND cause = 'disk full'").await,
        1,
        "/fail"
    );
    let ok = format!("GET /ping/{token} HTTP/1.1\r\nHost: x\r\n\r\n");
    assert_eq!(
        stored_after_hang_up(&app, ok, "kind = 'ok'").await,
        1,
        "/ping"
    );
}

#[tokio::test]
async fn shutdown_waits_for_a_ping_being_stored() {
    let app = TestApp::spawn("test").await;
    let (_tl, token) = setup(&app).await;
    let db = app.state.db.clone();
    let held = tokio::spawn(async move {
        db.write(async |_| {
            tokio::time::sleep(Duration::from_millis(600)).await;
            Ok(())
        })
        .await
    });
    tokio::time::sleep(Duration::from_millis(50)).await;
    let url = app.url(&format!("/ping/{token}"));
    let http = app.http.clone();
    let sent = tokio::spawn(async move { http.get(url).send().await.unwrap().status() });
    // The ping now waits on the writer inside `record`, holding its read guard.
    tokio::time::sleep(Duration::from_millis(200)).await;
    let early = monita::app::drain(&app.state.gate, Duration::from_millis(100)).await;
    assert!(early.is_none(), "drain waits for the store");
    held.await.unwrap().unwrap();
    assert_eq!(sent.await.unwrap(), 200);
    let late = monita::app::drain(&app.state.gate, Duration::from_secs(2)).await;
    assert!(late.is_some());
}

#[tokio::test]
async fn shutdown_does_not_wait_for_a_ping_body_upload() {
    use tokio::io::AsyncWriteExt;
    let app = TestApp::spawn("test").await;
    let (_tl, token) = setup(&app).await;
    let addr = app.base.trim_start_matches("http://").to_string();
    let mut tcp = tokio::net::TcpStream::connect(addr).await.unwrap();
    let head = format!(
        "POST /ping/{token}/fail HTTP/1.1\r\nHost: x\r\nContent-Length: 100000\r\n\r\ndisk"
    );
    tcp.write_all(head.as_bytes()).await.unwrap();
    // The cause read now waits for the rest of a stalled upload.
    tokio::time::sleep(Duration::from_millis(200)).await;
    let held = monita::app::drain(&app.state.gate, Duration::from_millis(500)).await;
    assert!(held.is_some(), "an upload holds no guard");
    drop(held);
    drop(tcp);
}

#[tokio::test]
async fn an_ok_ping_with_megabytes_of_body_is_answered_200() {
    let app = TestApp::spawn("test").await;
    let (_tl, token) = setup(&app).await;
    let res = app
        .http
        .post(app.url(&format!("/ping/{token}")))
        .body("x".repeat(4 * 1024 * 1024))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200, "never 413");
}

#[tokio::test]
async fn unknown_tokens_are_404_and_never_limited_per_address() {
    // v1.1 Q45: a token is random, and a limit per address would hold up every job behind it.
    let app = TestApp::spawn("test").await;
    for i in 0..70 {
        let res = ping(&app, reqwest::Method::GET, &format!("/ping/guess{i}")).await;
        assert_eq!(res.status(), 404);
    }
}

#[tokio::test]
async fn a_missed_deadline_is_reported_once_and_the_next_ping_clears_it() {
    let app = TestApp::spawn("test").await;
    let (tl, token) = setup(&app).await;
    app.state
        .db
        .write(async |c| {
            c.execute("UPDATE heartbeats SET deadline_at = 0", ())
                .await?;
            Ok(())
        })
        .await
        .unwrap();
    for _ in 0..30 {
        if count(
            &app,
            "SELECT count(*) FROM heartbeats WHERE deadline_at IS NULL",
        )
        .await
            == 1
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert_eq!(state_label(&app, &tl).await, "down");
    assert_eq!(
        ping(&app, reqwest::Method::GET, &format!("/ping/{token}"))
            .await
            .status(),
        200
    );
    assert_eq!(state_label(&app, &tl).await, "up");
}

#[tokio::test]
async fn an_unknown_token_gets_404() {
    let app = TestApp::spawn("test").await;
    let res = ping(&app, reqwest::Method::GET, "/ping/nosuchtoken").await;
    assert_eq!(res.status(), 404);
}

#[tokio::test]
async fn bad_period_is_refused() {
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    app.project(&tl, "Payments", "payments").await;
    let res = app
        .api_post(
            "/api/ui/projects/payments/heartbeats",
            json!({ "name": "x", "period_s": "7", "grace_s": "0", "severity": "warning" }),
            Some(&tl),
        )
        .await;
    assert_eq!(res.status(), 422);
    let fields = &json_of(res).await["error"]["fields"];
    assert!(fields["period_s"].is_string(), "{fields}");
}

async fn admitted(app: &TestApp, token: &str) -> heartbeat::Admitted {
    let p = heartbeat::Ping {
        token,
        kind: heartbeat::PingKind::Fail,
        source_ip: None,
    };
    match heartbeat::admit(&app.state.db, &app.state.limiter, &p)
        .await
        .unwrap()
    {
        heartbeat::Admission::Admitted(a) => a,
        _ => panic!("not admitted"),
    }
}

#[tokio::test]
async fn a_ping_whose_heartbeat_goes_while_its_body_uploads_is_not_stored() {
    let app = TestApp::spawn("test").await;
    let (_, token) = setup(&app).await;
    let a = admitted(&app, &token).await;
    heartbeat::delete(&app.state.db, BASE_URL, 1, 1)
        .await
        .unwrap();
    let stored = heartbeat::record(&app.state.db, BASE_URL, a, None)
        .await
        .unwrap();
    assert!(!stored, "the route answers 404");
    assert_eq!(count(&app, "SELECT count(*) FROM pings").await, 0);
}

#[tokio::test]
async fn a_ping_admitted_before_a_reset_never_lands_on_a_new_heartbeat() {
    let app = TestApp::spawn("test").await;
    let (_, token) = setup(&app).await;
    let a = admitted(&app, &token).await;
    let res = app
        .http
        .post(app.url("/admin/reset-state"))
        .header("x-reset-token", RESET_TOKEN)
        .json(&serde_json::json!({"seed": "qa"}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 204);
    // The QA seed's TL is user 2; the new project and heartbeat reuse id 1.
    app.state
        .db
        .write(async |c| {
            c.execute(
                "INSERT INTO projects (id, owner_id, name, slug) VALUES (1, 2, 'New', 'new')",
                (),
            )
            .await?;
            Ok(())
        })
        .await
        .unwrap();
    let input = HeartbeatInput {
        name: "fresh".into(),
        ..HeartbeatInput::default()
    };
    let id = heartbeat::create(&app.state.db, 1, &input)
        .await
        .unwrap()
        .unwrap();
    let fresh = heartbeat::find(&app.state.db, 1, id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(id, 1, "the precondition: the id is reused");
    assert_ne!(fresh.token, token);
    let stored = heartbeat::record(&app.state.db, BASE_URL, a, Some("stale".into()))
        .await
        .unwrap();
    assert!(!stored, "the route answers 404");
    assert_eq!(count(&app, "SELECT count(*) FROM pings").await, 0);
}

#[tokio::test]
async fn a_fail_uses_the_heartbeat_as_it_is_when_stored() {
    let app = TestApp::spawn("test").await;
    let (_, token) = setup(&app).await;
    let a = admitted(&app, &token).await;
    app.state
        .db
        .write(async |c| {
            c.execute("UPDATE heartbeats SET severity = 'critical'", ())
                .await?;
            Ok(())
        })
        .await
        .unwrap();
    heartbeat::record(&app.state.db, BASE_URL, a, Some("disk full".into()))
        .await
        .unwrap();
    assert_eq!(
        count(
            &app,
            "SELECT count(*) FROM incidents WHERE severity = 'critical'"
        )
        .await,
        1,
        "the severity edited during the upload"
    );
}

/// v1.1 Q38: a paused heartbeat keeps its pings but opens nothing, misses no deadline, and
/// starts a fresh deadline when resumed.
#[tokio::test]
async fn a_paused_heartbeat_keeps_pings_opens_nothing_and_resumes_with_a_fresh_deadline() {
    let app = TestApp::spawn("test").await;
    let (tl, token) = setup(&app).await;
    let pause = async |paused: bool| {
        app.api_send(
            reqwest::Method::PUT,
            "/api/ui/projects/payments/heartbeats/1/paused",
            json!({ "paused": paused }),
            Some(&tl),
        )
        .await
        .status()
    };
    assert_eq!(pause(true).await, 204);
    assert_eq!(state_label(&app, &tl).await, "paused");
    let res = ping(&app, reqwest::Method::POST, &format!("/ping/{token}/fail")).await;
    assert_eq!(res.status(), 200);
    assert_eq!(count(&app, "SELECT count(*) FROM pings").await, 1, "kept");
    assert_eq!(count(&app, "SELECT count(*) FROM incidents").await, 0);
    let read = app.state.db.read().await.unwrap();
    heartbeat::sweep_missed(
        &app.state.db,
        &read,
        BASE_URL,
        monita::platform::now() + 10 * 86_400,
    )
    .await
    .unwrap();
    drop(read);
    assert_eq!(
        count(&app, "SELECT count(*) FROM incidents").await,
        0,
        "no deadline is missed"
    );

    assert_eq!(pause(false).await, 204);
    let deadline = count(
        &app,
        "SELECT deadline_at - period_s - grace_s FROM heartbeats",
    )
    .await;
    assert!(
        (deadline - monita::platform::now()).abs() <= 2,
        "the deadline starts from now"
    );
    assert_ne!(state_label(&app, &tl).await, "paused");
}
