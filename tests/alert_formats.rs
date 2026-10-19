//! v1.1 Q17 (#10): each severity's webhook in its format, through the alerts form and the outbox.

mod common;

use common::{TestApp, json_of};
use monita::checks::heartbeat::{self, HeartbeatInput};
use serde_json::{Value, json};

async fn setup(app: &TestApp) -> String {
    let tl = app.tl("rina@x.co").await;
    app.project(&tl, "Payments", "payments").await;
    tl
}

async fn save(app: &TestApp, tl: &str, severity: &str, form: Value) -> reqwest::Response {
    app.api_send(
        reqwest::Method::PUT,
        &format!("/api/ui/projects/payments/alerts/{severity}"),
        form,
        Some(tl),
    )
    .await
}

/// Every webhook row in the outbox, oldest first: where it goes and what it says.
async fn webhooks(app: &TestApp) -> Vec<(String, Value)> {
    monita::platform::db::query_all(
        &app.state.db.read().await.unwrap(),
        "SELECT recipient, body FROM outbox WHERE channel = 'webhook' ORDER BY id",
        (),
        |r| Ok((r.get::<String>(0)?, r.get::<String>(1)?)),
    )
    .await
    .unwrap()
    .into_iter()
    .map(|(to, body)| (to, serde_json::from_str(&body).unwrap()))
    .collect()
}

#[tokio::test]
async fn a_failing_job_alerts_discord_in_its_own_shape() {
    let app = TestApp::spawn("test").await;
    let tl = setup(&app).await;
    let route = json!({ "emails": "", "webhook_url": "https://discord.com/api/webhooks/1/abc",
                        "use_webhook": true, "webhook_format": "discord" });
    assert_eq!(save(&app, &tl, "warning", route).await.status(), 204);
    let input = HeartbeatInput {
        name: "nightly".into(),
        ..HeartbeatInput::default()
    };
    heartbeat::create(&app.state.db, 1, &input).await.unwrap();
    let token = heartbeat::find(&app.state.db, 1, 1)
        .await
        .unwrap()
        .unwrap()
        .token;
    let fail = app
        .http
        .post(app.url(&format!("/ping/{token}/fail")))
        .body("disk full")
        .send()
        .await
        .unwrap();
    assert_eq!(fail.status(), 200);

    let sent = webhooks(&app).await;
    assert_eq!(sent.len(), 1, "{sent:?}");
    let (to, body) = &sent[0];
    assert_eq!(to, "https://discord.com/api/webhooks/1/abc");
    assert!(
        body["content"]
            .as_str()
            .unwrap()
            .starts_with("[WARNING] Payments / nightly"),
        "{body}"
    );
    assert!(
        body["content"]
            .as_str()
            .unwrap()
            .contains("/projects/payments/incidents/")
    );
}

#[tokio::test]
async fn a_pagerduty_test_triggers_and_resolves_with_one_key() {
    let app = TestApp::spawn("test").await;
    let tl = setup(&app).await;
    let route = json!({ "emails": "", "webhook_url": "", "use_webhook": true,
                        "webhook_format": "pagerduty", "webhook_key": "R0utingKey1" });
    assert_eq!(save(&app, &tl, "critical", route).await.status(), 204);
    let test = app
        .api_post(
            "/api/ui/projects/payments/alerts/critical/test",
            json!({}),
            Some(&tl),
        )
        .await;
    assert_eq!(
        json_of(test).await["data"]["sent_to"],
        1,
        "PagerDuty only: the form emptied the emails"
    );

    let sent = webhooks(&app).await;
    let actions: Vec<&str> = sent
        .iter()
        .map(|(_, b)| b["event_action"].as_str().unwrap())
        .collect();
    assert_eq!(actions, ["trigger", "resolve"]);
    assert!(
        sent.iter()
            .all(|(to, b)| to == "https://events.pagerduty.com/v2/enqueue"
                && b["routing_key"] == "R0utingKey1")
    );
    assert_eq!(sent[0].1["dedup_key"], sent[1].1["dedup_key"]);
}

#[tokio::test]
async fn the_form_names_what_a_format_needs_and_a_supervisor_never_sees_the_key() {
    let app = TestApp::spawn("test").await;
    let tl = setup(&app).await;
    let field = async |form: Value| {
        let res = save(&app, &tl, "critical", form).await;
        assert_eq!(res.status(), 422);
        let err = json_of(res).await;
        err["error"]["fields"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect::<Vec<_>>()
    };
    let telegram = "https://api.telegram.org/bot1:x/sendMessage";
    assert_eq!(
        field(json!({ "webhook_url": telegram, "webhook_format": "telegram" })).await,
        ["webhook_key"]
    );
    assert_eq!(
        field(json!({ "webhook_url": "https://ntfy.sh/", "webhook_format": "ntfy" })).await,
        ["webhook_url"]
    );
    assert_eq!(
        field(json!({ "webhook_format": "sms" })).await,
        ["webhook_format"]
    );

    let ok = json!({ "webhook_url": telegram, "use_webhook": true, "webhook_format": "telegram", "webhook_key": "-1001234" });
    assert_eq!(save(&app, &tl, "critical", ok).await.status(), 204);
    let mine = json_of(
        app.api_get("/api/ui/projects/payments/alerts", Some(&tl))
            .await,
    )
    .await;
    assert_eq!(mine["data"][0]["webhook_format"], "telegram");
    assert_eq!(mine["data"][0]["webhook_key"], "-1001234");

    let admin = app.super_admin("admin@x.co").await;
    let id = monita::accounts::invite_tl(&app.state.db, common::BASE_URL, "sup@x.co")
        .await
        .unwrap();
    let sup = app
        .accept(id.strip_prefix(common::BASE_URL).unwrap(), common::PASSWORD)
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
            reqwest::Method::PUT,
            &format!("/api/ui/admin/users/{sup_id}/role"),
            json!({ "role": "supervisor" }),
            Some(&admin),
        )
        .await;
    assert_eq!(role.status(), 204);
    let seen = json_of(
        app.api_get("/api/ui/projects/payments/alerts", Some(&sup))
            .await,
    )
    .await;
    assert_eq!(seen["data"][0]["webhook_key"], "(hidden)");
    assert_eq!(seen["data"][0]["webhook_url"], "(hidden)");
}
