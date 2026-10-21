//! Monita's Telegram bot (v1.1 Q33), against a fake Bot API server: the admin setting, the
//! connect link, and delivery that adds the token only when it sends.

mod common;

use std::sync::{Arc, Mutex};

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::any,
};
use common::{BASE_URL, PASSWORD, TestApp, json_of};
use serde_json::{Value, json};

const TOKEN: &str = "123456:ABCdefGHIjklMNOpqrSTUvwxYZ_0123456";

/// What the fake Bot API has to give and what it heard.
#[derive(Default)]
struct Fake {
    updates: Vec<Value>,
    sent: Vec<(String, Value)>,
}

async fn fake_api(
    State(fake): State<Arc<Mutex<Fake>>>,
    Path((bot, method)): Path<(String, String)>,
    body: Option<Json<Value>>,
) -> (axum::http::StatusCode, Json<Value>) {
    if bot != format!("bot{TOKEN}") {
        return (
            axum::http::StatusCode::UNAUTHORIZED,
            Json(json!({ "ok": false, "error_code": 401, "description": "Unauthorized" })),
        );
    }
    let mut f = fake.lock().unwrap();
    let result = match method.as_str() {
        "getMe" => json!({ "id": 123456, "is_bot": true, "username": "MonitaTestBot" }),
        "getWebhookInfo" => json!({ "url": "" }),
        // Telegram keeps updates until a later offset confirms them; this fake hands each out once.
        "getUpdates" => Value::Array(std::mem::take(&mut f.updates)),
        "sendMessage" => {
            f.sent
                .push((bot.clone(), body.map(|b| b.0).unwrap_or_default()));
            json!({ "message_id": 1 })
        }
        _ => json!(null),
    };
    (
        axum::http::StatusCode::OK,
        Json(json!({ "ok": true, "result": result })),
    )
}

async fn spawn_fake() -> (String, Arc<Mutex<Fake>>) {
    let fake = Arc::new(Mutex::new(Fake::default()));
    let app = Router::new()
        .route("/{bot}/{method}", any(fake_api))
        .with_state(fake.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (format!("http://{addr}"), fake)
}

async fn set_token(app: &TestApp, cookie: &str, token: &str) -> reqwest::Response {
    app.api_send(
        reqwest::Method::PUT,
        "/api/ui/admin/telegram",
        json!({ "token": token }),
        Some(cookie),
    )
    .await
}

async fn save_route(app: &TestApp, cookie: &str, chat: &str) -> reqwest::Response {
    app.api_send(
        reqwest::Method::PUT,
        "/api/ui/projects/pay/alerts/critical",
        json!({ "emails": "", "use_webhook": true, "webhook_format": "telegram_bot", "webhook_key": chat }),
        Some(cookie),
    )
    .await
}

#[tokio::test]
async fn a_super_admin_sets_the_bot_and_nobody_reads_its_token() {
    let (api, _) = spawn_fake().await;
    let app = TestApp::spawn_with("test", &[("MONITA_TELEGRAM_API", &api)]).await;
    let admin = app.super_admin("admin@x.co").await;
    let tl = app.tl("rina@x.co").await;

    let bad = set_token(&app, &admin, "not a token").await;
    assert_eq!(bad.status(), 422);
    assert_eq!(
        json_of(bad).await["error"]["fields"]["token"],
        "Telegram refused this token. Copy it again from @BotFather."
    );
    let refused = set_token(&app, &admin, "999:ABCdefGHIjklMNOpqrSTUvwxYZ_0123456").await;
    assert_eq!(
        refused.status(),
        422,
        "Telegram answers 401 to an unknown token"
    );
    assert_eq!(set_token(&app, &tl, TOKEN).await.status(), 403);

    let ok = set_token(&app, &admin, TOKEN).await;
    assert_eq!(ok.status(), 200);
    assert_eq!(json_of(ok).await["data"]["bot"], "MonitaTestBot");
    let seen = app
        .api_get("/api/ui/telegram", Some(&tl))
        .await
        .text()
        .await
        .unwrap();
    assert_eq!(
        seen, r#"{"data":{"bot":"MonitaTestBot"}}"#,
        "the name, never the token"
    );

    assert_eq!(set_token(&app, &admin, "").await.status(), 200);
    assert_eq!(
        json_of(app.api_get("/api/ui/telegram", Some(&tl)).await).await["data"]["bot"],
        Value::Null
    );
}

#[tokio::test]
async fn a_connect_link_fills_the_chat_and_alerts_carry_no_token() {
    let (api, fake) = spawn_fake().await;
    let app = TestApp::spawn_with("test", &[("MONITA_TELEGRAM_API", &api)]).await;
    let admin = app.super_admin("admin@x.co").await;
    let tl = app.tl("rina@x.co").await;
    assert_eq!(app.project(&tl, "Pay", "pay").await, 201);
    assert_eq!(app.project(&tl, "Shop", "shop").await, 201);

    let before = save_route(&app, &tl, "4242").await;
    assert_eq!(before.status(), 422);
    assert_eq!(
        json_of(before).await["error"]["fields"]["webhook_format"],
        "Ask a super admin to set Monita's Telegram bot first, or use your own bot."
    );
    let link_path = "/api/ui/projects/pay/alerts/critical/telegram-link";
    assert_eq!(
        app.api_post(link_path, json!({}), Some(&tl)).await.status(),
        422
    );
    assert_eq!(set_token(&app, &admin, TOKEN).await.status(), 200);

    let link = json_of(app.api_post(link_path, json!({}), Some(&tl)).await).await;
    let private = link["data"]["private"].as_str().unwrap();
    let code = private
        .strip_prefix("https://t.me/MonitaTestBot?start=")
        .unwrap()
        .to_string();
    assert_eq!(
        link["data"]["group"],
        format!("https://t.me/MonitaTestBot?startgroup={code}")
    );
    assert!(link["data"]["qr"].as_str().unwrap().contains("<svg"));
    let shop = json_of(
        app.api_post(
            "/api/ui/projects/shop/alerts/warning/telegram-link",
            json!({}),
            Some(&tl),
        )
        .await,
    )
    .await;
    let shop_code = shop["data"]["private"]
        .as_str()
        .unwrap()
        .rsplit('=')
        .next()
        .unwrap()
        .to_string();

    let check = format!("{link_path}/check");
    assert_eq!(
        json_of(app.api_post(&check, json!({}), Some(&tl)).await).await["data"]["chat"],
        Value::Null,
        "nobody opened it yet"
    );
    fake.lock().unwrap().updates = vec![
        json!({ "update_id": 5, "message": { "text": format!("/start {shop_code}"), "chat": { "id": -100, "title": "Shop on call" } } }),
        json!({ "update_id": 6, "message": { "text": format!("/start {code}"), "chat": { "id": 4242, "username": "rina" } } }),
    ];
    let heard = json_of(app.api_post(&check, json!({}), Some(&tl)).await).await;
    assert_eq!(
        heard["data"]["chat"],
        json!({ "chat_id": "4242", "title": "@rina" })
    );
    assert_eq!(
        json_of(app.api_post(&check, json!({}), Some(&tl)).await).await["data"]["chat"],
        Value::Null,
        "a link is used once"
    );
    let shop_heard = json_of(
        app.api_post(
            "/api/ui/projects/shop/alerts/warning/telegram-link/check",
            json!({}),
            Some(&tl),
        )
        .await,
    )
    .await;
    assert_eq!(
        shop_heard["data"]["chat"]["title"], "Shop on call",
        "another project's link heard in the same read is kept for it"
    );

    assert_eq!(save_route(&app, &tl, "4242").await.status(), 204);
    let tested = app
        .api_post(
            "/api/ui/projects/pay/alerts/critical/test",
            json!({}),
            Some(&tl),
        )
        .await;
    assert_eq!(tested.status(), 200);
    let rows = monita::platform::db::query_all(
        &app.state.db.read().await.unwrap(),
        "SELECT channel, recipient, body FROM outbox WHERE channel = 'telegram'",
        (),
        |r| {
            Ok((
                r.get::<String>(0)?,
                r.get::<String>(1)?,
                r.get::<String>(2)?,
            ))
        },
    )
    .await
    .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].1, "4242");
    assert!(!rows[0].2.contains(TOKEN), "the queued body holds no token");
    let route = json_of(app.api_get("/api/ui/projects/pay/alerts", Some(&tl)).await).await;
    assert!(!route.to_string().contains(TOKEN));

    monita::notify::deliver_due(&app.state).await.unwrap();
    let sent = std::mem::take(&mut fake.lock().unwrap().sent);
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].0, format!("bot{TOKEN}"), "delivery adds the token");
    assert_eq!(sent[0].1["chat_id"], "4242");
}

#[tokio::test]
async fn a_supervisor_reads_the_bot_name_and_cannot_open_links() {
    let (api, _) = spawn_fake().await;
    let app = TestApp::spawn_with("test", &[("MONITA_TELEGRAM_API", &api)]).await;
    let admin = app.super_admin("admin@x.co").await;
    let tl = app.tl("rina@x.co").await;
    assert_eq!(app.project(&tl, "Pay", "pay").await, 201);
    assert_eq!(set_token(&app, &admin, TOKEN).await.status(), 200);
    let res = app
        .api_post(
            "/api/ui/admin/users",
            json!({ "email": "sup@x.co", "role": "supervisor" }),
            Some(&admin),
        )
        .await;
    let link = json_of(res).await["data"]["link"]
        .as_str()
        .unwrap()
        .to_string();
    let sup = app
        .accept(link.strip_prefix(BASE_URL).unwrap(), PASSWORD)
        .await;
    assert_eq!(
        json_of(app.api_get("/api/ui/telegram", Some(&sup)).await).await["data"]["bot"],
        "MonitaTestBot"
    );
    assert_eq!(set_token(&app, &sup, TOKEN).await.status(), 403);
    let open = app
        .api_post(
            "/api/ui/projects/pay/alerts/critical/telegram-link",
            json!({}),
            Some(&sup),
        )
        .await;
    assert_eq!(open.status(), 403);
}
