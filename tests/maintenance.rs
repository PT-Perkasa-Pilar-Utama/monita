//! v1.1 Q18 (#11): maintenance windows. The check results carry their own times, so each test
//! moves the clock by the times it records, not by waiting.

mod common;

use common::{TestApp, json_of};
use monita::{
    checks::{self, CheckResult, MonitorInput, heartbeat},
    platform::{db::query_i64, now},
};
use serde_json::{Value, json};

const LIST: &str = "/api/ui/projects/payments/maintenance";

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

/// A tech lead, the project, and one public monitor that opens an incident on `fails` failures.
async fn setup(app: &TestApp, fails: i64) -> String {
    let tl = app.tl("rina@x.co").await;
    app.project(&tl, "Payments", "payments").await;
    let input = MonitorInput {
        name: "Checkout".into(),
        url: "https://x.invalid/".into(),
        is_public: true,
        fails_to_open: fails,
        ..MonitorInput::defaults()
    };
    checks::create_monitor(&app.state.db, 1, &input)
        .await
        .unwrap();
    // The scheduler stays away: the test records every result itself.
    sql(app, "UPDATE monitors SET next_check_at = 9999999999").await;
    tl
}

async fn record(app: &TestApp, ok: bool, at: i64) {
    let m = checks::find_monitor(&app.state.db, 1, 1)
        .await
        .unwrap()
        .unwrap();
    let result = CheckResult {
        checked_at: at,
        ok,
        status_code: Some(if ok { 200 } else { 503 }),
        latency_ms: 10,
        error: (!ok).then(|| "HTTP 503".into()),
        cert_expires_at: None,
    };
    checks::record_check(&app.state.db, common::BASE_URL, &m, &result)
        .await
        .unwrap();
}

/// Plans a window over `items` from now for an hour; returns its id.
async fn plan(app: &TestApp, tl: &str, items: &[&str], message: &str) -> i64 {
    let t = now();
    let res = app
        .api_post(
            LIST,
            json!({ "title": "Database upgrade", "message": message, "starts_at": t,
                    "ends_at": t + 3600, "items": items }),
            Some(tl),
        )
        .await;
    assert_eq!(res.status(), 201);
    json_of(res).await["data"]["id"].as_i64().unwrap()
}

const OPEN: &str = "SELECT count(*) FROM incidents WHERE resolved_at IS NULL";
const SENT: &str = "SELECT count(*) FROM outbox";

#[tokio::test]
async fn a_window_holds_back_incidents_and_alerts_and_the_first_failure_after_it_opens_one() {
    let app = TestApp::spawn("test").await;
    let tl = setup(&app, 2).await;
    let invite = count(&app, SENT).await;
    plan(&app, &tl, &["monitor:1"], "").await;
    let t = now() + 5;
    for i in 0..3 {
        record(&app, false, t + i).await;
    }
    assert_eq!(count(&app, "SELECT count(*) FROM check_results").await, 3);
    assert_eq!(count(&app, OPEN).await, 0, "no incident in the window");
    assert_eq!(count(&app, SENT).await, invite, "no alert in the window");

    sql(
        &app,
        &format!("UPDATE maintenance_windows SET ends_at = {}", t + 10),
    )
    .await;
    record(&app, false, t + 20).await;
    assert_eq!(
        count(&app, OPEN).await,
        1,
        "the streak kept counting, so one more failure opens it"
    );
    assert!(count(&app, SENT).await > invite, "and the alert goes out");
}

#[tokio::test]
async fn an_incident_open_before_the_window_stays_open_and_closes_as_usual() {
    let app = TestApp::spawn("test").await;
    let tl = setup(&app, 1).await;
    let t = now();
    record(&app, false, t - 100).await;
    assert_eq!(count(&app, OPEN).await, 1);
    let opened = count(&app, SENT).await;
    plan(&app, &tl, &["monitor:1"], "").await;
    record(&app, false, t + 5).await;
    assert_eq!(count(&app, OPEN).await, 1);

    let status = json_of(app.get("/api/ui/public/payments/status", None).await).await;
    assert_eq!(
        status["data"]["status"]["items"][0]["state"], "down",
        "an open incident outranks the window"
    );

    record(&app, true, t + 6).await;
    assert_eq!(count(&app, OPEN).await, 0);
    assert!(
        count(&app, SENT).await > opened,
        "the resolved alert still goes out"
    );
}

#[tokio::test]
async fn a_heartbeat_missed_in_a_window_opens_its_incident_when_the_window_ends() {
    let app = TestApp::spawn("test").await;
    let tl = setup(&app, 2).await;
    let input = heartbeat::HeartbeatInput {
        name: "nightly".into(),
        ..heartbeat::HeartbeatInput::default()
    };
    heartbeat::create(&app.state.db, 1, &input).await.unwrap();
    let invite = count(&app, SENT).await;
    plan(&app, &tl, &["heartbeat:1"], "").await;
    let t = now() + 5;
    let end = t + 100;
    sql(&app, &format!(
        "UPDATE maintenance_windows SET ends_at = {end}; UPDATE heartbeats SET deadline_at = {}",
        t - 1
    ))
    .await;
    let sweep = async |at: i64| {
        let read = app.state.db.read().await.unwrap();
        heartbeat::sweep_missed(&app.state.db, &read, common::BASE_URL, at)
            .await
            .unwrap()
    };

    sweep(t).await;
    assert_eq!(count(&app, OPEN).await, 0);
    assert_eq!(
        count(&app, "SELECT deadline_at FROM heartbeats").await,
        t + 60,
        "the miss waits a minute, then is looked at again"
    );
    sweep(t + 60).await;
    assert_eq!(count(&app, "SELECT deadline_at FROM heartbeats").await, end);
    assert_eq!(count(&app, OPEN).await, 0);

    sweep(end).await;
    assert_eq!(
        count(&app, OPEN).await,
        1,
        "the job stayed silent past the window"
    );
    assert!(count(&app, SENT).await > invite);
}

#[tokio::test]
async fn a_failure_reported_in_a_window_opens_its_incident_after_it_unless_the_job_recovers() {
    let app = TestApp::spawn("test").await;
    let tl = setup(&app, 2).await;
    for name in ["backup", "export"] {
        let input = heartbeat::HeartbeatInput {
            name: name.into(),
            ..heartbeat::HeartbeatInput::default()
        };
        heartbeat::create(&app.state.db, 1, &input).await.unwrap();
    }
    plan(&app, &tl, &["heartbeat:1", "heartbeat:2"], "").await;
    let token = async |id: i64| {
        let c = app.state.db.read().await.unwrap();
        let mut rows = c
            .query("SELECT token FROM heartbeats WHERE id = ?1", [id])
            .await
            .unwrap();
        let t: String = rows.next().await.unwrap().unwrap().get(0).unwrap();
        t
    };
    let ping = async |path: String| {
        app.http
            .post(app.url(&path))
            .body("disk full")
            .send()
            .await
            .unwrap()
            .status()
    };
    let (backup, export) = (token(1).await, token(2).await);
    assert_eq!(ping(format!("/ping/{backup}/fail")).await, 200);
    assert_eq!(ping(format!("/ping/{export}/fail")).await, 200);
    assert_eq!(count(&app, OPEN).await, 0, "nothing opens in the window");
    // The export job recovers before the window ends; its next deadline is a period away.
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    assert_eq!(ping(format!("/ping/{export}")).await, 200);
    let held = count(&app, "SELECT deadline_at FROM heartbeats WHERE id = 1").await;
    assert!(held <= now() + 60, "the failure is held, not lost: {held}");

    sql(
        &app,
        &format!("UPDATE maintenance_windows SET ends_at = {}", now() - 1),
    )
    .await;
    let read = app.state.db.read().await.unwrap();
    heartbeat::sweep_missed(&app.state.db, &read, common::BASE_URL, now() + 61)
        .await
        .unwrap();
    assert_eq!(
        count(&app, OPEN).await,
        1,
        "the backup stayed failed past the window"
    );
    let cause: String = {
        let c = app.state.db.read().await.unwrap();
        let mut rows = c
            .query("SELECT cause FROM incidents WHERE resolved_at IS NULL", ())
            .await
            .unwrap();
        rows.next().await.unwrap().unwrap().get(0).unwrap()
    };
    assert_eq!(
        cause, "disk full",
        "the incident carries what the job reported"
    );
    assert_eq!(
        count(
            &app,
            "SELECT count(*) FROM incident_items WHERE source_id = 2"
        )
        .await,
        0,
        "the export recovered in time"
    );
}

#[tokio::test]
async fn the_status_page_shows_maintenance_and_the_public_message() {
    let app = TestApp::spawn("test").await;
    let tl = setup(&app, 2).await;
    let hidden = MonitorInput {
        name: "Internal".into(),
        url: "https://y.invalid/".into(),
        ..MonitorInput::defaults()
    };
    checks::create_monitor(&app.state.db, 1, &hidden)
        .await
        .unwrap();
    plan(&app, &tl, &["monitor:2"], "Private work.").await;
    plan(
        &app,
        &tl,
        &["monitor:1", "monitor:2"],
        "Checkout pauses until 02:00.",
    )
    .await;

    let page = json_of(app.get("/api/ui/public/payments/status", None).await).await;
    let status = &page["data"]["status"];
    assert_eq!(status["overall"], "maintenance");
    assert_eq!(status["items"][0]["state"], "maintenance");
    let windows = status["maintenance"].as_array().unwrap();
    assert_eq!(windows.len(), 1, "a window over hidden items stays private");
    assert_eq!(windows[0]["message"], "Checkout pauses until 02:00.");
    assert_eq!(windows[0]["items"], json!(["Checkout"]));
    let days = status["items"][0]["days"].as_array().unwrap();
    assert_eq!(days.last().unwrap()["downtime_s"], 0);

    let monitors = json_of(
        app.api_get("/api/ui/projects/payments/monitors", Some(&tl))
            .await,
    )
    .await;
    assert!(
        monitors.to_string().contains("\"maintenance\""),
        "the team sees the state too: {monitors}"
    );
}

#[tokio::test]
async fn only_a_scheduled_window_deletes_and_only_an_active_one_ends() {
    let app = TestApp::spawn("test").await;
    let tl = setup(&app, 2).await;
    let active = plan(&app, &tl, &["monitor:1"], "").await;
    let t = now();
    let later = json_of(
        app.api_post(
            LIST,
            json!({ "title": "Network move", "starts_at": t + 7200, "ends_at": t + 9000, "items": ["monitor:1"] }),
            Some(&tl),
        )
        .await,
    )
    .await["data"]["id"]
        .as_i64()
        .unwrap();
    let del = async |id: i64| {
        app.api_send(
            reqwest::Method::DELETE,
            &format!("{LIST}/{id}"),
            json!({}),
            Some(&tl),
        )
        .await
        .status()
    };
    let end = async |id: i64| {
        app.api_post(&format!("{LIST}/{id}/end"), json!({}), Some(&tl))
            .await
            .status()
    };

    assert_eq!(del(active).await, 422, "an active window ends instead");
    assert_eq!(end(later).await, 422, "a scheduled one cannot end now");
    assert_eq!(end(active).await, 204);
    assert_eq!(end(active).await, 422, "an ended window is read only");
    assert_eq!(del(active).await, 422);
    assert_eq!(del(later).await, 204);
    assert_eq!(del(999).await, 404);

    let list = json_of(app.api_get(LIST, Some(&tl)).await).await;
    let windows = list["data"]["windows"].as_array().unwrap();
    assert_eq!(windows.len(), 1);
    assert_eq!(windows[0]["state"], "ended");
    assert_eq!(windows[0]["items"][0]["name"], "Checkout");

    let other = app.tl("omar@x.co").await;
    assert_eq!(app.api_get(LIST, Some(&other)).await.status(), 404);
}

#[tokio::test]
async fn the_form_names_every_bad_field() {
    let app = TestApp::spawn("test").await;
    let tl = setup(&app, 2).await;
    let res = app
        .api_post(LIST, json!({ "items": ["monitor:99"] }), Some(&tl))
        .await;
    assert_eq!(res.status(), 422);
    let err: Value = json_of(res).await;
    let mut fields: Vec<&String> = err["error"]["fields"].as_object().unwrap().keys().collect();
    fields.sort();
    assert_eq!(fields, ["ends_at", "items", "starts_at", "title"]);
}

#[tokio::test]
async fn resolving_by_hand_forgets_a_held_failure() {
    let app = TestApp::spawn("test").await;
    setup(&app, 2).await;
    let input = heartbeat::HeartbeatInput {
        name: "backup".into(),
        ..heartbeat::HeartbeatInput::default()
    };
    heartbeat::create(&app.state.db, 1, &input).await.unwrap();
    sql(&app, "UPDATE heartbeats SET held_cause = 'disk full'").await;
    app.state
        .db
        .write(async |c| checks::rearm_heartbeat(c, 1, now()).await)
        .await
        .unwrap();
    assert_eq!(
        count(
            &app,
            "SELECT count(*) FROM heartbeats WHERE held_cause IS NULL"
        )
        .await,
        1
    );
}
