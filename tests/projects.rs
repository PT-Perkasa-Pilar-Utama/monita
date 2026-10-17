//! US-05 to US-08, and the ref-id API that the key rules guard (AC-23.01 to AC-23.03).

mod common;

use common::{TestApp, location};
use serde_json::Value;

async fn project(app: &TestApp, cookie: &str, name: &str, slug: &str) -> reqwest::Response {
    app.post_form("/projects", &[("name", name), ("slug", slug)], Some(cookie))
        .await
}

/// Creates a key on the settings page and reads it back from the one-time notice.
async fn new_key(app: &TestApp, cookie: &str, slug: &str) -> String {
    let res = app
        .post_form(
            &format!("/projects/{slug}/keys"),
            &[("name", "backend-prod")],
            Some(cookie),
        )
        .await;
    assert_eq!(res.status(), 200);
    let html = res.text().await.unwrap();
    let start = html.find("id=\"new-key\">").expect("key not shown") + "id=\"new-key\">".len();
    html[start..]
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect()
}

async fn ref_id(app: &TestApp, key: Option<&str>, body: Value) -> reqwest::Response {
    let mut req = app.http.post(app.url("/api/v1/ref-ids")).json(&body);
    if let Some(k) = key {
        req = req.bearer_auth(k);
    }
    req.send().await.unwrap()
}

#[tokio::test]
async fn tl_creates_a_project_and_owns_it() {
    // AC-06.01
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    let res = project(&app, &tl, "Payments", "payments").await;
    assert_eq!(res.status(), 303);
    assert_eq!(location(&res), "/projects/payments/monitors");
    let settings = app
        .get("/projects/payments/settings", Some(&tl))
        .await
        .text()
        .await
        .unwrap();
    assert!(settings.contains("rina@x.co"));
}

#[tokio::test]
async fn every_project_page_links_to_its_public_status_page() {
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    project(&app, &tl, "Payments", "payments").await;
    for page in ["monitors", "incidents", "settings"] {
        let html = app
            .get(&format!("/projects/payments/{page}"), Some(&tl))
            .await
            .text()
            .await
            .unwrap();
        assert!(html.contains(r#"href="/status/payments""#), "{page}");
    }
}

#[tokio::test]
async fn a_slug_is_unique_and_well_formed() {
    // AC-06.02
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    project(&app, &tl, "Payments", "payments").await;
    assert_eq!(
        project(&app, &tl, "Payments 2", "payments").await.status(),
        422
    );
    assert_eq!(project(&app, &tl, "Bad", "Has Spaces").await.status(), 422);
}

#[tokio::test]
async fn another_tl_cannot_see_the_project() {
    let app = TestApp::spawn("test").await;
    let rina = app.tl("rina@x.co").await;
    let budi = app.tl("budi@x.co").await;
    project(&app, &rina, "Payments", "payments").await;
    assert_eq!(
        app.get("/projects/payments/settings", Some(&budi))
            .await
            .status(),
        404
    );
    assert!(
        !app.get("/", Some(&budi))
            .await
            .text()
            .await
            .unwrap()
            .contains("/projects/payments/")
    );
}

#[tokio::test]
async fn super_admin_reassigns_a_project_and_its_keys_stay() {
    // AC-05.01, AC-05.02, AC-05.03
    let app = TestApp::spawn("test").await;
    let admin = app.super_admin("admin@x.co").await;
    let rina = app.tl("rina@x.co").await;
    let budi = app.tl("budi@x.co").await;
    project(&app, &rina, "Payments", "payments").await;
    let key = new_key(&app, &rina, "payments").await;
    let budi_id = monita::accounts::list_users(&app.state.db)
        .await
        .unwrap()
        .into_iter()
        .find(|u| u.email == "budi@x.co")
        .unwrap()
        .id;

    let res = app
        .post_form(
            "/projects/payments/owner",
            &[("owner_id", &budi_id.to_string())],
            Some(&admin),
        )
        .await;
    assert_eq!(res.status(), 303);
    assert_eq!(
        app.get("/projects/payments/settings", Some(&budi))
            .await
            .status(),
        200
    );
    assert_eq!(
        app.get("/projects/payments/settings", Some(&rina))
            .await
            .status(),
        404
    );
    assert_eq!(
        ref_id(&app, Some(&key), Value::Null).await.status(),
        201,
        "project data stays"
    );
}

#[tokio::test]
async fn a_project_goes_only_to_an_active_tl() {
    let app = TestApp::spawn("test").await;
    let admin = app.super_admin("admin@x.co").await;
    let rina = app.tl("rina@x.co").await;
    app.tl("budi@x.co").await;
    project(&app, &rina, "Payments", "payments").await;
    let budi_id = monita::accounts::list_users(&app.state.db)
        .await
        .unwrap()
        .into_iter()
        .find(|u| u.email == "budi@x.co")
        .unwrap()
        .id;
    monita::accounts::disable(&app.state.db, budi_id)
        .await
        .unwrap();
    for owner in [budi_id.to_string(), "999".to_string()] {
        let res = app
            .post_form(
                "/projects/payments/owner",
                &[("owner_id", &owner)],
                Some(&admin),
            )
            .await;
        assert_eq!(res.status(), 404, "owner {owner}");
    }
    assert_eq!(
        app.get("/projects/payments/settings", Some(&rina))
            .await
            .status(),
        200,
        "rina still owns it"
    );
}

#[tokio::test]
async fn the_owner_list_offers_the_admin_and_marks_the_current_owner() {
    let app = TestApp::spawn("test").await;
    let admin = app.super_admin("admin@x.co").await;
    let rina = app.tl("rina@x.co").await;
    project(&app, &rina, "Payments", "payments").await;
    let html = app
        .get("/projects/payments/settings", Some(&admin))
        .await
        .text()
        .await
        .unwrap();
    let admin_at = html
        .find(">admin@x.co</option>")
        .expect("the admin is offered");
    let rina_at = html
        .find(" selected>rina@x.co</option>")
        .expect("the owner is selected");
    assert!(admin_at < rina_at, "the admin comes first");
}

#[tokio::test]
async fn a_tl_cannot_reassign() {
    let app = TestApp::spawn("test").await;
    let rina = app.tl("rina@x.co").await;
    project(&app, &rina, "Payments", "payments").await;
    let res = app
        .post_form(
            "/projects/payments/owner",
            &[("owner_id", "1")],
            Some(&rina),
        )
        .await;
    assert_eq!(res.status(), 403);
}

#[tokio::test]
async fn a_key_is_shown_once_works_and_stops_when_revoked() {
    // AC-07.01, AC-07.02, AC-23.01, AC-23.02, AC-23.03
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    project(&app, &tl, "Payments", "payments").await;
    let key = new_key(&app, &tl, "payments").await;
    assert_eq!(key.len(), 29, "mk_ + 26 base32 chars");
    let settings = app
        .get("/projects/payments/settings", Some(&tl))
        .await
        .text()
        .await
        .unwrap();
    assert!(!settings.contains(&key), "the full key appears only once");

    let reusable = ref_id(&app, Some(&key), serde_json::json!({})).await;
    assert_eq!(reusable.status(), 201);
    let body: Value = reusable.json().await.unwrap();
    assert!(body["data"]["ref_id"].as_str().unwrap().starts_with("rf_"));
    assert_eq!(body["data"]["single_use"], false);
    let once: Value = ref_id(&app, Some(&key), serde_json::json!({"single_use": true}))
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(once["data"]["single_use"], true);

    assert_eq!(ref_id(&app, None, Value::Null).await.status(), 401);
    assert_eq!(
        ref_id(&app, Some("mk_not-a-real-key"), Value::Null)
            .await
            .status(),
        401
    );

    let keys = monita::projects::list_keys(&app.state.db, 1).await.unwrap();
    app.post_form(
        &format!("/projects/payments/keys/{}/revoke", keys[0].id),
        &[],
        Some(&tl),
    )
    .await;
    let revoked = ref_id(&app, Some(&key), Value::Null).await;
    assert_eq!(revoked.status(), 401);
    let err: Value = revoked.json().await.unwrap();
    assert_eq!(err["error"]["code"], "unauthorized");
}

#[tokio::test]
async fn the_allowlist_is_off_by_default_and_blocks_other_ips() {
    // AC-08.01, AC-08.02
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    project(&app, &tl, "Payments", "payments").await;
    let key = new_key(&app, &tl, "payments").await;
    let settings = app
        .get("/projects/payments/settings", Some(&tl))
        .await
        .text()
        .await
        .unwrap();
    assert!(settings.contains("IP allowlist</h2><span class=\"cap\">Off"));
    assert_eq!(ref_id(&app, Some(&key), Value::Null).await.status(), 201);

    let res = app
        .post_form(
            "/projects/payments/allowlist",
            &[("cidrs", "10.20.0.0/16")],
            Some(&tl),
        )
        .await;
    assert_eq!(res.status(), 303);
    let blocked = ref_id(&app, Some(&key), Value::Null).await;
    assert_eq!(
        blocked.status(),
        403,
        "the test client calls from 127.0.0.1"
    );

    app.post_form(
        "/projects/payments/allowlist",
        &[("cidrs", "10.20.0.0/16\n127.0.0.0/8")],
        Some(&tl),
    )
    .await;
    assert_eq!(ref_id(&app, Some(&key), Value::Null).await.status(), 201);

    let bad = app
        .post_form(
            "/projects/payments/allowlist",
            &[("cidrs", "intranet")],
            Some(&tl),
        )
        .await;
    assert_eq!(bad.status(), 422);
}

#[tokio::test]
async fn ref_id_body_is_optional_but_must_be_json_when_given() {
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    project(&app, &tl, "Payments", "payments").await;
    let key = new_key(&app, &tl, "payments").await;
    let empty = app
        .http
        .post(app.url("/api/v1/ref-ids"))
        .bearer_auth(&key)
        .send()
        .await
        .unwrap();
    assert_eq!(empty.status(), 201);
    let bad = app
        .http
        .post(app.url("/api/v1/ref-ids"))
        .bearer_auth(&key)
        .body("single_use=yes")
        .send()
        .await
        .unwrap();
    assert_eq!(bad.status(), 400);
    let err: Value = bad.json().await.unwrap();
    assert_eq!(err["error"]["code"], "invalid_request");
}
