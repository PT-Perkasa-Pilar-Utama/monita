//! US-12 to US-14. The clock is moved through deadline_at.

mod common;

use std::time::Duration;

use common::{BASE_URL, TestApp, location};
use monita::{
    checks::heartbeat::{self, HeartbeatInput},
    platform::db::query_i64,
};

async fn count(app: &TestApp, sql: &str) -> i64 {
    query_i64(&app.state.db.read().await.unwrap(), sql, ())
        .await
        .unwrap()
        .unwrap_or(0)
}

async fn setup(app: &TestApp) -> (String, String) {
    let tl = app.tl("rina@x.co").await;
    app.post_form(
        "/projects",
        &[("name", "Payments"), ("slug", "payments")],
        Some(&tl),
    )
    .await;
    let id = heartbeat::create(
        &app.state.db,
        1,
        &HeartbeatInput {
            name: "nightly".into(),
            ..HeartbeatInput::default()
        },
    )
    .await
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
    let html = app
        .get("/projects/payments/heartbeats/1", Some(tl))
        .await
        .text()
        .await
        .unwrap();
    let pill = &html[html.find("<span class=\"state").expect("state pill")..];
    let label = &pill[pill.find("></span>").unwrap() + "></span>".len()..];
    label[..label.find("</span>").unwrap()].to_string()
}

#[tokio::test]
async fn tl_creates_a_heartbeat_and_sees_its_ping_url() {
    // AC-12.01, AC-12.02
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    app.post_form(
        "/projects",
        &[("name", "Payments"), ("slug", "payments")],
        Some(&tl),
    )
    .await;
    let form = app
        .get("/projects/payments/heartbeats/new", Some(&tl))
        .await
        .text()
        .await
        .unwrap();
    for field in [
        "name=\"name\"",
        "name=\"period_s\"",
        "name=\"grace_s\"",
        "name=\"severity\"",
        "name=\"is_public\"",
    ] {
        assert!(form.contains(field), "missing {field}");
    }
    let res = app
        .post_form(
            "/projects/payments/heartbeats",
            &[
                ("name", "nightly-settlement"),
                ("period_s", "3600"),
                ("grace_s", "300"),
                ("severity", "critical"),
            ],
            Some(&tl),
        )
        .await;
    assert_eq!(res.status(), 303);
    let html = app
        .get(&location(&res), Some(&tl))
        .await
        .text()
        .await
        .unwrap();
    let token = heartbeat::find(&app.state.db, 1, 1)
        .await
        .unwrap()
        .unwrap()
        .token;
    assert_eq!(token.len(), 26);
    assert!(html.contains(&format!("{BASE_URL}/ping/{token}")));
    assert!(html.contains("Waiting"));
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
    assert_eq!(state_label(&app, &tl).await, "Up");
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
    assert_eq!(state_label(&app, &tl).await, "Up");
}

#[tokio::test]
async fn a_fail_ping_marks_the_heartbeat_down_at_once() {
    // AC-14.01
    let app = TestApp::spawn("test").await;
    let (tl, token) = setup(&app).await;
    let res = ping(&app, reqwest::Method::POST, &format!("/ping/{token}/fail")).await;
    assert_eq!(res.status(), 200);
    assert_eq!(state_label(&app, &tl).await, "Down");
    assert_eq!(
        count(&app, "SELECT count(*) FROM pings WHERE kind = 'fail'").await,
        1
    );
}

#[tokio::test]
async fn a_fail_right_after_a_success_ping_is_still_heard() {
    let app = TestApp::spawn("test").await;
    let (tl, token) = setup(&app).await;
    let ok = ping(&app, reqwest::Method::GET, &format!("/ping/{token}")).await;
    assert_eq!(ok.status(), 200);
    let fail = ping(&app, reqwest::Method::POST, &format!("/ping/{token}/fail")).await;
    assert_eq!(fail.status(), 200);
    assert_eq!(state_label(&app, &tl).await, "Down");
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
async fn a_fail_body_is_the_cause_up_to_1_kb_and_413_past_it() {
    let app = TestApp::spawn("test").await;
    let (_, token) = setup(&app).await;
    let url = app.url(&format!("/ping/{token}/fail"));
    let big = app
        .http
        .post(&url)
        .body("x".repeat(1025))
        .send()
        .await
        .unwrap();
    assert_eq!(big.status(), 413);
    let ok = app
        .http
        .post(&url)
        .body("disk full on /var")
        .send()
        .await
        .unwrap();
    assert_eq!(ok.status(), 200);
    assert_eq!(
        count(
            &app,
            "SELECT count(*) FROM pings WHERE kind = 'fail' AND cause = 'disk full on /var'"
        )
        .await,
        1
    );
}

#[tokio::test]
async fn guessing_tokens_from_one_ip_gets_429() {
    let app = TestApp::spawn("test").await;
    // 121 misses span at most two minute windows, so one window sees more than 60.
    let mut statuses = Vec::new();
    for i in 0..121 {
        let res = ping(&app, reqwest::Method::GET, &format!("/ping/guess{i}")).await;
        statuses.push(res.status().as_u16());
    }
    assert_eq!(statuses[0], 404);
    assert!(statuses.contains(&429), "misses are throttled per IP");
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
    assert_eq!(state_label(&app, &tl).await, "Down");
    assert_eq!(
        ping(&app, reqwest::Method::GET, &format!("/ping/{token}"))
            .await
            .status(),
        200
    );
    assert_eq!(state_label(&app, &tl).await, "Up");
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
    app.post_form(
        "/projects",
        &[("name", "Payments"), ("slug", "payments")],
        Some(&tl),
    )
    .await;
    let res = app
        .post_form(
            "/projects/payments/heartbeats",
            &[
                ("name", "x"),
                ("period_s", "7"),
                ("grace_s", "0"),
                ("severity", "warning"),
            ],
            Some(&tl),
        )
        .await;
    assert_eq!(res.status(), 422);
}
