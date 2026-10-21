//! A user's own settings: the password and the picture.

mod common;

use common::{BASE_URL, PASSWORD, TestApp, json_of};
use serde_json::json;

const PATH: &str = "/api/ui/me/password";
/// Exactly the minimum: 8 characters.
const NEW: &str = "8chars!!";

async fn change(app: &TestApp, cookie: &str, current: &str, new: &str) -> reqwest::Response {
    app.api_post(
        PATH,
        json!({ "current": current, "new": new }),
        Some(cookie),
    )
    .await
}

/// A super admin invites a supervisor, who sets a password. Returns the supervisor's cookie.
async fn supervisor(app: &TestApp) -> String {
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
    app.accept(link.strip_prefix(BASE_URL).unwrap(), PASSWORD)
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
        field(change(&app, &here, PASSWORD, "7chars!").await, "new").await,
        "Use at least 8 characters."
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
    let sup = supervisor(&app).await;
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

const PNG_1X1: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==";

async fn set_avatar(app: &TestApp, cookie: &str, avatar: &str) -> reqwest::Response {
    app.api_send(
        reqwest::Method::PUT,
        "/api/ui/me/avatar",
        json!({ "avatar": avatar }),
        Some(cookie),
    )
    .await
}

async fn avatar_at(app: &TestApp, cookie: &str) -> serde_json::Value {
    json_of(app.api_get("/api/ui/me", Some(cookie)).await).await["data"]["avatar_at"].clone()
}

#[tokio::test]
async fn a_picture_is_stored_served_to_the_team_and_removed() {
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    let other = app.tl("budi@x.co").await;
    let me = json_of(app.api_get("/api/ui/me", Some(&tl)).await).await["data"]["id"].clone();
    let path = format!("/users/{me}/avatar");
    assert!(avatar_at(&app, &tl).await.is_null());
    assert_eq!(app.get(&path, Some(&tl)).await.status(), 404);

    let svg = "PHN2ZyB4bWxucz0naHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmcnLz4=";
    assert_eq!(
        field(set_avatar(&app, &tl, svg).await, "avatar").await,
        "Use a PNG, JPEG or WebP picture."
    );

    let png = format!("data:image/png;base64,{PNG_1X1}");
    assert_eq!(set_avatar(&app, &tl, &png).await.status(), 204);
    assert!(avatar_at(&app, &tl).await.is_i64());
    let res = app.get(&path, Some(&tl)).await;
    assert_eq!(res.status(), 200);
    assert_eq!(res.headers()["content-type"], "image/png");
    assert_eq!(res.bytes().await.unwrap().len(), 70);
    assert_eq!(
        app.get(&path, Some(&other)).await.status(),
        200,
        "another user sees it"
    );
    assert_eq!(
        app.get(&path, None).await.status(),
        303,
        "no session goes to sign-in"
    );

    assert_eq!(set_avatar(&app, &tl, "").await.status(), 204);
    assert!(avatar_at(&app, &tl).await.is_null());
    assert_eq!(app.get(&path, Some(&tl)).await.status(), 404);
}

#[tokio::test]
async fn a_supervisor_sets_their_own_picture() {
    let app = TestApp::spawn("test").await;
    let sup = supervisor(&app).await;
    assert_eq!(set_avatar(&app, &sup, PNG_1X1).await.status(), 204);
}
