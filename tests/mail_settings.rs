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

#[tokio::test]
async fn a_test_email_goes_out_at_once_and_says_whether_it_went() {
    let app = TestApp::spawn("test").await;
    let admin = app.super_admin("root@x.co").await;
    let tl = app.tl("rina@x.co").await;
    let path = "/api/ui/admin/mail/test-send";
    let to = json!({ "to": "ops@x.co" });
    assert_eq!(
        app.api_post(path, to.clone(), Some(&tl)).await.status(),
        403
    );
    let bad = app
        .api_post(path, json!({ "to": "not an address" }), Some(&admin))
        .await;
    assert_eq!(bad.status(), 422);
    assert!(json_of(bad).await["error"]["fields"]["to"].is_string());

    let res = json_of(app.api_post(path, to, Some(&admin)).await).await;
    assert_eq!(res["data"]["ok"], true);
    let sent = app.state.mailer.sent();
    assert_eq!(
        (sent.len(), sent[0].to.as_str()),
        (1, "ops@x.co"),
        "sent now, not queued"
    );
}

#[tokio::test]
async fn a_connection_test_tries_the_form_and_says_why_it_failed() {
    let app = TestApp::spawn("test").await;
    let admin = app.super_admin("root@x.co").await;
    let path = "/api/ui/admin/mail/test-connection";
    let mut form = form("");
    form["port"] = json!("70000");
    assert_eq!(
        app.api_post(path, form.clone(), Some(&admin))
            .await
            .status(),
        422
    );

    // Nothing listens on port 9, so the connection is refused at once.
    form["host"] = json!("127.0.0.1");
    form["port"] = json!("9");
    form["security"] = json!("none");
    let res = json_of(app.api_post(path, form, Some(&admin)).await).await;
    assert_eq!(res["data"]["ok"], false);
    assert!(!res["data"]["error"].as_str().unwrap().is_empty(), "{res}");
    assert_eq!(
        mail(&app, &admin).await.0["source"],
        "none",
        "a test saves nothing"
    );
}
