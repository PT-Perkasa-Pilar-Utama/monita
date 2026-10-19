//! The System page's mail server form: saved settings replace `MONITA_SMTP_URL`, and the
//! password never comes back.

mod common;

use common::{TestApp, json_of};
use serde_json::{Value, json};

async fn put(app: &TestApp, who: &str, body: Value) -> u16 {
    app.api_send(reqwest::Method::PUT, "/api/ui/admin/mail", body, Some(who))
        .await
        .status()
        .as_u16()
}

/// The System view's `mail`, and the whole body as text.
async fn mail(app: &TestApp, who: &str) -> (Value, String) {
    let res = app.api_get("/api/ui/admin/system", Some(who)).await;
    let text = res.text().await.unwrap();
    let v: Value = serde_json::from_str(&text).unwrap();
    (v["data"]["mail"].clone(), text)
}

fn form(password: &str) -> Value {
    json!({ "host": "mail.company.co.id", "port": "587", "security": "starttls", "user": "monita",
            "password": password, "from": "Monita <monita@company.co.id>" })
}

#[tokio::test]
async fn an_admin_saves_the_mail_server_and_the_password_stays_on_the_server() {
    let app = TestApp::spawn("test").await;
    let admin = app.super_admin("root@x.co").await;
    let tl = app.tl("rina@x.co").await;
    assert_eq!(mail(&app, &admin).await.0["source"], "none");

    assert_eq!(
        put(&app, &tl, form("hunter2")).await,
        403,
        "super admin only"
    );
    let mut bad = form("hunter2");
    bad["port"] = json!("70000");
    let res = app
        .api_send(
            reqwest::Method::PUT,
            "/api/ui/admin/mail",
            bad,
            Some(&admin),
        )
        .await;
    assert_eq!(res.status(), 422);
    assert!(json_of(res).await["error"]["fields"]["port"].is_string());

    assert_eq!(put(&app, &admin, form("hunter2")).await, 204);
    let (m, text) = mail(&app, &admin).await;
    assert_eq!(m["source"], "saved");
    assert_eq!(m["server"], "smtp://mail.company.co.id:587");
    assert_eq!(m["user"], "monita");
    assert_eq!(m["has_password"], true);
    assert!(
        !text.contains("hunter2"),
        "the password never leaves the server"
    );

    assert_eq!(put(&app, &admin, form("")).await, 204);
    assert_eq!(
        mail(&app, &admin).await.0["has_password"],
        true,
        "blank keeps it"
    );
    let mut remove = form("");
    remove["remove_password"] = json!(true);
    remove["user"] = json!("");
    assert_eq!(put(&app, &admin, remove).await, 204);
    assert_eq!(mail(&app, &admin).await.0["has_password"], false);

    let res = app
        .api_send(
            reqwest::Method::DELETE,
            "/api/ui/admin/mail",
            json!({}),
            Some(&admin),
        )
        .await;
    assert_eq!(res.status(), 204);
    assert_eq!(
        mail(&app, &admin).await.0["source"],
        "none",
        "back to the log"
    );
}
