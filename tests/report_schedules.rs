//! Weekly and monthly summary reports by email (v1.2 Q59): a switch and addresses per cadence,
//! one email per address once a period closes, and the PDF attached.

mod common;

use common::{BASE_URL, TestApp, json_of};
use reqwest::Method;
use serde_json::{Value, json};

const WEEK: i64 = 7 * 86_400;

async fn put(app: &TestApp, cookie: &str, cadence: &str, body: Value) -> reqwest::Response {
    app.api_send(
        Method::PUT,
        &format!("/api/ui/projects/payments/report-schedules/{cadence}"),
        body,
        Some(cookie),
    )
    .await
}

async fn schedules(app: &TestApp, cookie: &str) -> Value {
    json_of(
        app.api_get("/api/ui/projects/payments/report-schedules", Some(cookie))
            .await,
    )
    .await["data"]
        .clone()
}

async fn sql_i64(app: &TestApp, sql: &str) -> i64 {
    let c = app.state.db.read().await.unwrap();
    monita::platform::db::query_i64(&c, sql, ())
        .await
        .unwrap()
        .unwrap_or(0)
}

#[tokio::test]
async fn a_schedule_is_off_until_saved_and_needs_good_addresses_to_be_on() {
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    app.project(&tl, "Payments", "payments").await;
    assert_eq!(
        schedules(&app, &tl).await,
        json!([
            { "cadence": "weekly", "enabled": false, "emails": [] },
            { "cadence": "monthly", "enabled": false, "emails": [] },
        ])
    );

    let res = put(
        &app,
        &tl,
        "weekly",
        json!({ "enabled": true, "emails": "" }),
    )
    .await;
    assert_eq!(res.status(), 422);
    assert_eq!(
        json_of(res).await["error"]["fields"]["emails"],
        "Enter at least one email address."
    );
    let res = put(
        &app,
        &tl,
        "weekly",
        json!({ "enabled": true, "emails": "boss@x.co\nnot an address" }),
    )
    .await;
    assert_eq!(res.status(), 422);
    let many: Vec<String> = (0..21).map(|i| format!("p{i}@x.co")).collect();
    let res = put(
        &app,
        &tl,
        "weekly",
        json!({ "enabled": true, "emails": many.join("\n") }),
    )
    .await;
    assert_eq!(
        json_of(res).await["error"]["fields"]["emails"],
        "Enter 20 addresses at most."
    );

    let res = put(
        &app,
        &tl,
        "weekly",
        json!({ "enabled": true, "emails": " Boss@X.co \nops@x.co, boss@x.co" }),
    )
    .await;
    assert_eq!(res.status(), 204);
    assert_eq!(
        put(
            &app,
            &tl,
            "monthly",
            json!({ "enabled": false, "emails": "" })
        )
        .await
        .status(),
        204,
        "off needs no address"
    );
    assert_eq!(
        put(
            &app,
            &tl,
            "daily",
            json!({ "enabled": false, "emails": "" })
        )
        .await
        .status(),
        404
    );
    assert_eq!(
        schedules(&app, &tl).await[0],
        json!({ "cadence": "weekly", "enabled": true, "emails": ["boss@x.co", "ops@x.co"] })
    );
}

#[tokio::test]
async fn only_the_owner_or_a_super_admin_changes_a_schedule() {
    let app = TestApp::spawn("test").await;
    let admin = app.super_admin("boss@x.co").await;
    let rina = app.tl("rina@x.co").await;
    let budi = app.tl("budi@x.co").await;
    app.project(&rina, "Payments", "payments").await;
    let off = json!({ "enabled": false, "emails": "" });
    assert_eq!(put(&app, &budi, "weekly", off.clone()).await.status(), 404);
    assert_eq!(put(&app, &admin, "weekly", off.clone()).await.status(), 204);

    let res = app
        .api_post(
            "/api/ui/admin/users",
            json!({ "email": "sari@x.co", "role": "supervisor" }),
            Some(&admin),
        )
        .await;
    let link = json_of(res).await["data"]["link"]
        .as_str()
        .unwrap()
        .to_string();
    let sup = app
        .accept(link.strip_prefix(BASE_URL).unwrap(), common::PASSWORD)
        .await;
    assert_eq!(
        app.api_get("/api/ui/projects/payments/report-schedules", Some(&sup))
            .await
            .status(),
        200
    );
    assert_eq!(put(&app, &sup, "weekly", off).await.status(), 403);
}

#[tokio::test]
async fn a_closed_week_goes_out_once_to_each_address_with_its_pdf() {
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    app.project(&tl, "Payments", "payments").await;
    let res = put(
        &app,
        &tl,
        "weekly",
        json!({ "enabled": true, "emails": "boss@x.co\nops@x.co" }),
    )
    .await;
    assert_eq!(res.status(), 204);

    let now = monita::platform::now();
    // The week that closed before it was turned on counts as sent.
    assert_eq!(
        monita::reports::run_due(&app.state.db, BASE_URL, now)
            .await
            .unwrap(),
        0
    );
    let next = now + WEEK;
    assert_eq!(
        monita::reports::run_due(&app.state.db, BASE_URL, next)
            .await
            .unwrap(),
        2
    );
    assert_eq!(
        monita::reports::run_due(&app.state.db, BASE_URL, next)
            .await
            .unwrap(),
        0,
        "a period goes out once"
    );
    assert_eq!(
        sql_i64(
            &app,
            "SELECT count(*) FROM outbox WHERE attachment IS NOT NULL"
        )
        .await,
        2
    );

    // Due now, so this test's delivery pass sends them.
    app.state
        .db
        .write(async |c| {
            c.execute("UPDATE outbox SET next_attempt_at = 0", ())
                .await?;
            Ok(())
        })
        .await
        .unwrap();
    monita::notify::deliver_due(&app.state).await.unwrap();
    let sent = app.state.mailer.sent();
    let reports: Vec<_> = sent
        .iter()
        .filter(|e| e.subject.starts_with("Payments weekly report, "))
        .collect();
    assert_eq!(reports.len(), 2);
    let pdf = reports[0].attachment.as_ref().expect("the PDF rides along");
    assert!(pdf.name.starts_with("payments-") && pdf.name.ends_with(".pdf"));
    assert!(pdf.bytes.starts_with(b"%PDF-1.4"));
    assert!(reports[0].body.contains("/projects/payments"));
    assert_eq!(
        sql_i64(
            &app,
            "SELECT count(*) FROM outbox WHERE attachment IS NOT NULL"
        )
        .await,
        0,
        "a sent email drops its PDF"
    );

    // Off: the next week sends nothing.
    put(
        &app,
        &tl,
        "weekly",
        json!({ "enabled": false, "emails": "boss@x.co" }),
    )
    .await;
    assert_eq!(
        monita::reports::run_due(&app.state.db, BASE_URL, next + WEEK)
            .await
            .unwrap(),
        0
    );
}
