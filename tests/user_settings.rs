//! A user's own settings: changing the password.

mod common;

use common::{BASE_URL, PASSWORD, TestApp, json_of};
use serde_json::json;

const PATH: &str = "/api/ui/me/password";
const NEW: &str = "a-new-long-password";

async fn change(app: &TestApp, cookie: &str, current: &str, new: &str) -> reqwest::Response {
    app.api_post(
        PATH,
        json!({ "current": current, "new": new }),
        Some(cookie),
    )
    .await
}

async fn field(res: reqwest::Response, name: &str) -> String {
    assert_eq!(res.status(), 422);
    json_of(res).await["error"]["fields"][name]
        .as_str()
        .unwrap()
        .to_string()
}

#[tokio::test]
async fn a_new_password_needs_the_current_one_and_ends_other_sessions() {
    let app = TestApp::spawn("test").await;
    let here = app.tl("rina@x.co").await;
    let elsewhere = app.login("rina@x.co", PASSWORD).await;

    assert_eq!(
        field(change(&app, &here, "", NEW).await, "current").await,
        "Enter your current password."
    );
    assert_eq!(
        field(change(&app, &here, "not-my-password", NEW).await, "current").await,
        "This is not your current password."
    );
    assert_eq!(
        field(change(&app, &here, PASSWORD, "short").await, "new").await,
        "Use at least 12 characters."
    );

    assert_eq!(change(&app, &here, PASSWORD, NEW).await.status(), 204);
    assert_eq!(
        app.api_get("/api/ui/me", Some(&here)).await.status(),
        200,
        "this session stays"
    );
    assert_eq!(
        app.api_get("/api/ui/me", Some(&elsewhere)).await.status(),
        401,
        "every other session ends"
    );
    let old = app
        .api_post(
            "/api/ui/login",
            json!({ "email": "rina@x.co", "password": PASSWORD }),
            None,
        )
        .await;
    assert_eq!(old.status(), 422, "the old password stops working");
    app.login("rina@x.co", NEW).await;
}

#[tokio::test]
async fn a_supervisor_changes_their_own_password() {
    let app = TestApp::spawn("test").await;
    let admin = app.super_admin("admin@x.co").await;
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
    assert_eq!(change(&app, &sup, PASSWORD, NEW).await.status(), 204);
    app.login("sup@x.co", NEW).await;
}

#[tokio::test]
async fn ten_tries_in_a_window_then_even_the_right_password_waits() {
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    for _ in 0..10 {
        assert_eq!(
            change(&app, &tl, "wrong-guess-123", NEW).await.status(),
            422
        );
    }
    let res = change(&app, &tl, PASSWORD, NEW).await;
    assert_eq!(res.status(), 429);
    assert_eq!(json_of(res).await["error"]["code"], "too_many_tries");
}
