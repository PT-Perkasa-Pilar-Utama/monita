//! v1.1 Q17 (#10): an alert card's webhook in its format, through the card form and the outbox.

mod common;

use common::{TestApp, json_of};
use monita::checks::heartbeat::{self, HeartbeatInput};
use serde_json::{Value, json};

async fn setup(app: &TestApp) -> String {
    let tl = app.tl("rina@x.co").await;
    app.project(&tl, "Payments", "payments").await;
    tl
}

async fn add(app: &TestApp, tl: &str, form: Value) -> reqwest::Response {
    app.api_post("/api/ui/projects/payments/alert-cards", form, Some(tl))
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
async fn a_failing_job_alerts_discord_as_an_embed() {
    let app = TestApp::spawn("test").await;
    let tl = setup(&app).await;
    let card = json!({ "kind": "discord", "name": "Team chat", "warning": true,
                       "url": "https://discord.com/api/webhooks/1/abc" });
    assert_eq!(add(&app, &tl, card).await.status(), 201);
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
    let embed = &body["embeds"][0];
    assert!(
        embed["title"]
            .as_str()
            .unwrap()
            .starts_with("🟠 Warning: nightly"),
        "{body}"
    );
    assert!(
        embed["url"]
            .as_str()
            .unwrap()
            .contains("/projects/payments/incidents/")
    );
    assert_eq!(embed["fields"][0]["value"], "Payments");
    assert_eq!(
        embed["fields"][1]["value"], "disk full",
        "the job's own words"
    );
}

#[tokio::test]
async fn a_pagerduty_test_triggers_and_resolves_with_one_key() {
    let app = TestApp::spawn("test").await;
    let tl = setup(&app).await;
    let card = json!({ "kind": "pagerduty", "name": "Pager", "critical": true,
                       "key": "R0utingKey1" });
    let id = json_of(add(&app, &tl, card).await).await["data"]["id"].clone();
    let test = app
        .api_post(
            &format!("/api/ui/projects/payments/alert-cards/{id}/test"),
            json!({}),
            Some(&tl),
        )
        .await;
    assert_eq!(
        json_of(test).await["data"]["sent_to"],
        1,
        "the test goes to this card alone, not the owner's email"
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
    let field = async |mut form: Value| {
        form["name"] = json!("Card");
        form["critical"] = json!(true);
        let res = add(&app, &tl, form).await;
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
        field(json!({ "url": telegram, "kind": "telegram" })).await,
        ["key"]
    );
    assert_eq!(
        field(json!({ "url": "https://ntfy.sh/", "kind": "ntfy" })).await,
        ["url"]
    );
    assert_eq!(field(json!({ "kind": "sms" })).await, ["kind"]);
    assert_eq!(
        field(json!({ "kind": "email", "emails": "" })).await,
        ["emails"]
    );
    let no_level = add(
        &app,
        &tl,
        json!({ "kind": "email", "name": "X", "emails": "a@x.co" }),
    )
    .await;
    assert_eq!(
        json_of(no_level).await["error"]["fields"]["levels"],
        "Pick critical, warning or both."
    );

    let ok = json!({ "kind": "telegram", "name": "Ops", "critical": true, "url": telegram, "key": "-1001234" });
    assert_eq!(add(&app, &tl, ok).await.status(), 201);
    let mine = app.alert_cards(&tl, "payments").await;
    let card = |cards: &Value| {
        cards
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["kind"] == "telegram")
            .unwrap()
            .clone()
    };
    assert_eq!(card(&mine)["key"], "-1001234");

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
    let seen = card(&app.alert_cards(&sup, "payments").await);
    assert_eq!(seen["key"], "(hidden)");
    assert_eq!(seen["url"], "(hidden)");
}

/// v1.1 Q36: the ticket webhook speaks a format too, PagerDuty aside.
#[tokio::test]
async fn ticket_alerts_speak_a_format_but_not_pagerduty() {
    let app = TestApp::spawn("test").await;
    let tl = setup(&app).await;
    let put = async |body: Value| {
        app.api_send(
            reqwest::Method::PUT,
            "/api/ui/projects/payments/ticket-alerts",
            body,
            Some(&tl),
        )
        .await
    };
    let field = async |res: reqwest::Response| {
        assert_eq!(res.status(), 422);
        json_of(res).await["error"]["fields"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect::<Vec<_>>()
    };
    let pd =
        json!({ "use_webhook": true, "webhook_format": "pagerduty", "webhook_key": "R0utingKey1" });
    assert_eq!(field(put(pd).await).await, ["webhook_format"]);
    let bot =
        json!({ "use_webhook": true, "webhook_format": "telegram_bot", "webhook_key": "4242" });
    assert_eq!(
        field(put(bot).await).await,
        ["webhook_format"],
        "no global bot is set"
    );

    let discord = json!({ "digest": true, "use_webhook": true, "webhook_format": "discord",
                          "webhook_url": "https://discord.com/api/webhooks/2/t" });
    assert_eq!(put(discord).await.status(), 204);
    app.state
        .db
        .write(async |c| {
            monita::notify::send_ticket(c, 1, 7, "Receipt shows <b>", common::BASE_URL).await
        })
        .await
        .unwrap();
    let sent = webhooks(&app).await;
    let (to, body) = sent.last().unwrap();
    assert_eq!(to, "https://discord.com/api/webhooks/2/t");
    assert_eq!(
        body["embeds"][0]["title"],
        "🔵 New ticket: #7 Receipt shows <b>"
    );
    assert!(
        body["embeds"][0]["url"]
            .as_str()
            .unwrap()
            .ends_with("/projects/payments/tickets/7")
    );
}
