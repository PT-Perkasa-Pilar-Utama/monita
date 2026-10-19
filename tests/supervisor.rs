//! The supervisor role: sees what a super admin sees, changes nothing, and never gets a value
//! that would let them act through another door.

mod common;

use common::{BASE_URL, TestApp, json_of};
use reqwest::Method;
use serde_json::{Value, json};

/// A super admin invites a supervisor, who sets a password. Returns the supervisor's cookie.
async fn supervisor(app: &TestApp, admin: &str, email: &str) -> String {
    let res = app
        .api_post(
            "/api/ui/admin/users",
            json!({ "email": email, "role": "supervisor" }),
            Some(admin),
        )
        .await;
    assert_eq!(res.status(), 201);
    let link = json_of(res).await["data"]["link"]
        .as_str()
        .unwrap()
        .to_string();
    app.accept(link.strip_prefix(BASE_URL).unwrap(), common::PASSWORD)
        .await
}

async fn user_id(app: &TestApp, email: &str) -> i64 {
    let conn = app.state.db.read().await.unwrap();
    monita::platform::db::query_i64(&conn, "SELECT id FROM users WHERE email = ?1", [email])
        .await
        .unwrap()
        .unwrap()
}

fn monitor(headers: &str, body: &str) -> Value {
    json!({ "name": "API", "url": "http://127.0.0.1:9/", "interval_s": "60", "timeout_s": "5",
            "expected_status": "2xx", "severity": "warning", "is_public": true, "method": "POST",
            "headers": headers, "body": body })
}

#[tokio::test]
async fn a_supervisor_reads_every_project_and_the_admin_pages_but_changes_nothing() {
    let app = TestApp::spawn("test").await;
    let admin = app.super_admin("admin@x.co").await;
    let tl = app.tl("rina@x.co").await;
    app.project(&tl, "Payments", "payments").await;
    let sup = supervisor(&app, &admin, "sari@x.co").await;

    let me = json_of(app.api_get("/api/ui/me", Some(&sup)).await).await;
    assert_eq!(me["data"]["role"], "supervisor");
    assert_eq!(me["data"]["is_admin"], false);
    assert_eq!(me["data"]["sees_all"], true);
    assert_eq!(me["data"]["read_only"], true);

    let list = json_of(app.api_get("/api/ui/projects", Some(&sup)).await).await;
    assert_eq!(
        list["data"]["projects"][0]["slug"], "payments",
        "another owner's project"
    );
    for path in [
        "/api/ui/projects/payments",
        "/api/ui/projects/payments/monitors",
        "/api/ui/admin/users",
        "/api/ui/admin/system",
        "/api/ui/incidents",
    ] {
        assert_eq!(app.api_get(path, Some(&sup)).await.status(), 200, "{path}");
    }

    let writes = [
        (
            Method::POST,
            "/api/ui/projects/payments/monitors",
            monitor("", ""),
        ),
        (
            Method::POST,
            "/api/ui/projects",
            json!({ "name": "Mine", "homepage_url": "https://a.example.com" }),
        ),
        (Method::PUT, "/api/ui/admin/limits", json!({})),
        (
            Method::POST,
            "/api/ui/admin/users",
            json!({ "email": "x@x.co" }),
        ),
        (
            Method::DELETE,
            "/api/ui/projects/payments",
            json!({ "confirm": "payments" }),
        ),
    ];
    for (method, path, body) in writes {
        let res = app.api_send(method.clone(), path, body, Some(&sup)).await;
        assert_eq!(res.status(), 403, "{method} {path}");
    }
    let conn = app.state.db.read().await.unwrap();
    let projects = monita::platform::db::query_i64(&conn, "SELECT count(*) FROM projects", ())
        .await
        .unwrap();
    assert_eq!(projects, Some(1), "nothing was created or deleted");
    drop(conn);

    let out = app.api_post("/api/ui/logout", json!({}), Some(&sup)).await;
    assert_eq!(out.status(), 204, "signing out still works");
}

#[tokio::test]
async fn a_supervisor_never_sees_a_ping_url_a_header_value_or_a_request_body() {
    let app = TestApp::spawn("test").await;
    let admin = app.super_admin("admin@x.co").await;
    let tl = app.tl("rina@x.co").await;
    app.project(&tl, "Payments", "payments").await;
    let sup = supervisor(&app, &admin, "sari@x.co").await;
    let made = app
        .api_post(
            "/api/ui/projects/payments/monitors",
            monitor("X-Key: s3cret", "{\"token\":\"t0p\"}"),
            Some(&tl),
        )
        .await;
    assert_eq!(made.status(), 201);
    let id = json_of(made).await["data"]["id"].as_i64().unwrap();
    let hb = app
        .api_post(
            "/api/ui/projects/payments/heartbeats",
            json!({ "name": "nightly", "period_s": "3600", "grace_s": "300", "severity": "warning" }),
            Some(&tl),
        )
        .await;
    let hb_id = json_of(hb).await["data"]["id"].as_i64().unwrap();

    let path = format!("/api/ui/projects/payments/monitors/{id}");
    let seen = json_of(app.api_get(&path, Some(&sup)).await).await;
    assert_eq!(seen["data"]["monitor"]["headers"], "X-Key: (hidden)");
    assert_eq!(seen["data"]["monitor"]["body"], "(hidden)");
    let listed = json_of(
        app.api_get("/api/ui/projects/payments/monitors", Some(&sup))
            .await,
    )
    .await;
    assert_eq!(listed["data"]["monitors"][0]["headers"], "X-Key: (hidden)");
    let owner = json_of(app.api_get(&path, Some(&tl)).await).await;
    assert_eq!(owner["data"]["monitor"]["headers"], "X-Key: s3cret");

    let hb_path = format!("/api/ui/projects/payments/heartbeats/{hb_id}");
    let seen = json_of(app.api_get(&hb_path, Some(&sup)).await).await;
    assert!(seen["data"]["ping_url"].is_null());
    let owner = json_of(app.api_get(&hb_path, Some(&tl)).await).await;
    assert!(
        owner["data"]["ping_url"]
            .as_str()
            .unwrap()
            .contains("/ping/")
    );
}

#[tokio::test]
async fn a_project_owner_cannot_become_a_supervisor_and_a_supervisor_is_never_an_owner() {
    let app = TestApp::spawn("test").await;
    let admin = app.super_admin("admin@x.co").await;
    let tl = app.tl("rina@x.co").await;
    app.project(&tl, "Payments", "payments").await;
    app.tl("budi@x.co").await;
    let set = |email_id: i64, role: &'static str| {
        let admin = admin.clone();
        let app = &app;
        async move {
            app.api_send(
                Method::PUT,
                &format!("/api/ui/admin/users/{email_id}/role"),
                json!({ "role": role }),
                Some(&admin),
            )
            .await
        }
    };

    let refused = set(user_id(&app, "rina@x.co").await, "supervisor").await;
    assert_eq!(refused.status(), 422);
    assert!(json_of(refused).await["error"]["fields"]["role"].is_string());

    let budi = user_id(&app, "budi@x.co").await;
    assert_eq!(
        set(budi, "supervisor").await.status(),
        204,
        "budi owns nothing"
    );
    let owners = json_of(
        app.api_get("/api/ui/projects/payments/owners", Some(&admin))
            .await,
    )
    .await;
    let emails: Vec<&str> = owners["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| o["email"].as_str().unwrap())
        .collect();
    assert!(!emails.contains(&"budi@x.co"), "{emails:?}");
    let to_budi = app
        .api_send(
            Method::PUT,
            "/api/ui/projects/payments/owner",
            json!({ "owner_email": "budi@x.co" }),
            Some(&admin),
        )
        .await;
    assert_ne!(
        to_budi.status(),
        204,
        "a supervisor cannot be given a project"
    );
}
