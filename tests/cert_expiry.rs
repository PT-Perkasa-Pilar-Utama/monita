//! v1.1 Q25 (#8): an HTTPS monitor warns before its certificate expires, once per certificate,
//! through the warning route, without an incident. The check results carry the expiry the test
//! chooses, as an HTTPS check would read it from the server's leaf certificate.

mod common;

use common::{TestApp, json_of};
use monita::{
    checks::{self, CheckResult},
    platform::{db::query_i64, now},
};
use serde_json::{Value, json};

const MONITORS: &str = "/api/ui/projects/payments/monitors";
const DAY: i64 = 86_400;

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

fn form(kind: &str, url: &str, days: &str) -> Value {
    json!({ "name": "Checkout", "kind": kind, "url": url, "interval_s": "60", "timeout_s": "10",
            "expected_status": "2xx", "severity": "critical", "cert_warn_days": days })
}

/// A tech lead and the project; returns the tech lead's cookie.
async fn setup(app: &TestApp) -> String {
    let tl = app.tl("rina@x.co").await;
    app.project(&tl, "Payments", "payments").await;
    tl
}

/// One passing check on monitor 1 that saw a certificate expiring at `expires_at`.
async fn check_sees(app: &TestApp, expires_at: i64) {
    let m = checks::find_monitor(&app.state.db, 1, 1)
        .await
        .unwrap()
        .unwrap();
    let result = CheckResult {
        checked_at: now(),
        ok: true,
        status_code: Some(200),
        latency_ms: 10,
        error: None,
        cert_expires_at: Some(expires_at),
    };
    checks::record_check(&app.state.db, common::BASE_URL, &m, &result)
        .await
        .unwrap();
}

const CERT_MAILS: &str =
    "SELECT count(*) FROM outbox WHERE channel = 'email' AND subject LIKE '[CERT]%'";

#[tokio::test]
async fn the_warning_needs_an_https_monitor_and_a_listed_number_of_days() {
    let app = TestApp::spawn("test").await;
    let tl = setup(&app).await;
    let res = app
        .api_post(
            MONITORS,
            form("http", "http://pay.example.com", "14"),
            Some(&tl),
        )
        .await;
    assert_eq!(res.status(), 422);
    let body = json_of(res).await;
    assert!(
        body["error"]["fields"]["cert_warn_days"]
            .as_str()
            .unwrap()
            .contains("https://")
    );

    let res = app
        .api_post(
            MONITORS,
            form("http", "https://pay.example.com", "10"),
            Some(&tl),
        )
        .await;
    assert_eq!(res.status(), 422);
    assert!(json_of(res).await["error"]["fields"]["cert_warn_days"].is_string());

    let res = app
        .api_post(
            MONITORS,
            form("http", "https://pay.example.com", "14"),
            Some(&tl),
        )
        .await;
    assert_eq!(res.status(), 201);
    let detail = json_of(app.api_get(&format!("{MONITORS}/1"), Some(&tl)).await).await;
    assert_eq!(detail["data"]["monitor"]["cert_warn_days"], 14);

    // A TCP check has no certificate; the setting is dropped, not refused.
    let res = app
        .api_post(MONITORS, form("tcp", "db.internal:5432", "14"), Some(&tl))
        .await;
    assert_eq!(res.status(), 201);
    assert_eq!(
        count(
            &app,
            "SELECT count(*) FROM monitors WHERE kind = 'tcp' AND cert_warn_days IS NULL"
        )
        .await,
        1
    );
}

#[tokio::test]
async fn a_certificate_warns_once_inside_its_window_and_a_renewal_warns_again_later() {
    let app = TestApp::spawn("test").await;
    let tl = setup(&app).await;
    let res = app
        .api_post(
            MONITORS,
            form("http", "https://pay.example.com", "14"),
            Some(&tl),
        )
        .await;
    assert_eq!(res.status(), 201);
    sql(&app, "UPDATE monitors SET next_check_at = 9999999999").await;

    let far = now() + 60 * DAY;
    check_sees(&app, far).await;
    assert_eq!(count(&app, CERT_MAILS).await, 0, "outside the window");
    assert_eq!(
        count(
            &app,
            &format!("SELECT count(*) FROM monitors WHERE cert_expires_at = {far}")
        )
        .await,
        1
    );

    let soon = now() + 10 * DAY;
    check_sees(&app, soon).await;
    check_sees(&app, soon).await;
    assert_eq!(
        count(&app, CERT_MAILS).await,
        1,
        "one warning per certificate"
    );
    assert_eq!(
        count(&app, "SELECT count(*) FROM incidents").await,
        0,
        "no incident"
    );
    let subject: String = {
        let c = app.state.db.read().await.unwrap();
        let mut rows = c
            .query(
                "SELECT subject FROM outbox WHERE subject LIKE '[CERT]%'",
                (),
            )
            .await
            .unwrap();
        rows.next().await.unwrap().unwrap().get(0).unwrap()
    };
    assert!(
        subject.contains("Checkout") && subject.contains("expires in"),
        "{subject}"
    );

    // Renewed for 90 days: quiet now, and it warns again when the new one nears its end.
    check_sees(&app, now() + 90 * DAY).await;
    assert_eq!(count(&app, CERT_MAILS).await, 1);
    check_sees(&app, now() + 5 * DAY).await;
    assert_eq!(count(&app, CERT_MAILS).await, 2);
}

#[tokio::test]
async fn no_warning_when_the_monitor_alerts_are_off_or_the_warning_is_off() {
    let app = TestApp::spawn("test").await;
    let tl = setup(&app).await;
    let mut body = form("http", "https://pay.example.com", "30");
    body["alert"] = json!(false);
    assert_eq!(app.api_post(MONITORS, body, Some(&tl)).await.status(), 201);
    sql(&app, "UPDATE monitors SET next_check_at = 9999999999").await;
    check_sees(&app, now() + 3 * DAY).await;
    assert_eq!(count(&app, CERT_MAILS).await, 0, "alerts off");

    sql(&app, "UPDATE monitors SET alert = 1, cert_warn_days = NULL").await;
    check_sees(&app, now() + 3 * DAY).await;
    assert_eq!(count(&app, CERT_MAILS).await, 0, "warning off");
    assert_eq!(
        count(
            &app,
            "SELECT count(*) FROM monitors WHERE cert_expires_at IS NOT NULL"
        )
        .await,
        1,
        "the expiry is still kept"
    );
}
