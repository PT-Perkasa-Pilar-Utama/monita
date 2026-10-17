//! US-05 to US-08, and the ref-id API that the key rules guard (AC-23.01 to AC-23.03).

mod common;

use common::{TestApp, location};
use serde_json::Value;

async fn project(app: &TestApp, cookie: &str, name: &str, slug: &str) -> reqwest::Response {
    app.project(cookie, name, slug).await
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
    assert!(location(&res).starts_with("/projects/payments-"));
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
async fn monita_makes_a_unique_slug_from_the_name() {
    // AC-06.02
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    let mut slugs = Vec::new();
    for _ in 0..2 {
        let res = app
            .post_form("/projects", &[("name", "Customer Portal")], Some(&tl))
            .await;
        assert_eq!(res.status(), 303);
        let slug = location(&res)
            .trim_start_matches("/projects/")
            .trim_end_matches("/monitors")
            .to_string();
        let (base, suffix) = slug.rsplit_once('-').unwrap();
        assert_eq!(base, "customer-portal");
        assert_eq!(suffix.len(), 6);
        let status = app.get(&format!("/status/{slug}"), None).await.status();
        assert_eq!(status, 200, "AC-06.03");
        slugs.push(slug);
    }
    assert_ne!(slugs[0], slugs[1]);
    let blank = app
        .post_form("/projects", &[("name", "  ")], Some(&tl))
        .await;
    assert_eq!(blank.status(), 422);
}

#[tokio::test]
async fn the_overview_sums_up_the_project_and_lists_its_facts() {
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    project(&app, &tl, "Payments", "payments").await;
    new_key(&app, &tl, "payments").await;
    let html = app
        .get("/projects/payments", Some(&tl))
        .await
        .text()
        .await
        .unwrap();
    assert!(html.contains("<h1>Payments</h1>"));
    assert!(
        html.contains("Add your first monitor.") && html.contains("No ticket waits in Triage.")
    );
    assert!(html.contains("Owned by rina@x.co") && html.contains("1 active API key"));
    assert!(html.contains("Every system is up"));
}

#[tokio::test]
async fn another_tl_cannot_see_the_project() {
    let app = TestApp::spawn("test").await;
    let rina = app.tl("rina@x.co").await;
    let budi = app.tl("budi@x.co").await;
    project(&app, &rina, "Payments", "payments").await;
    for path in ["/projects/payments", "/projects/payments/settings"] {
        assert_eq!(app.get(path, Some(&budi)).await.status(), 404, "{path}");
    }
    assert!(
        !app.get("/", Some(&budi))
            .await
            .text()
            .await
            .unwrap()
            .contains("/projects/payments")
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

    let res = app
        .post_form(
            "/projects/payments/owner",
            &[("owner_email", " Budi@X.co ")],
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
    for owner in ["budi@x.co", "nobody@x.co", ""] {
        let res = app
            .post_form(
                "/projects/payments/owner",
                &[("owner_email", owner)],
                Some(&admin),
            )
            .await;
        assert_eq!(res.status(), 422, "owner {owner}");
        let html = res.text().await.unwrap();
        assert!(html.contains("Pick the owner from the list"));
        assert!(html.contains(r#"id="reassign" aria-labelledby="reassign-title" data-open"#));
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
        .find(r#"<option value="admin@x.co">"#)
        .expect("the admin is offered");
    let rina_at = html
        .find(r#"<option value="rina@x.co">"#)
        .expect("the tech lead is offered");
    assert!(admin_at < rina_at, "the admin comes first");
    assert!(
        html.contains(r#"name="owner_email" type="email" list="owners" value="rina@x.co""#),
        "the field starts at the current owner"
    );
}

#[tokio::test]
async fn a_super_admin_names_the_owner_and_a_tl_always_owns_what_they_create() {
    let app = TestApp::spawn("test").await;
    let admin = app.super_admin("admin@x.co").await;
    let rina = app.tl("rina@x.co").await;
    let budi = app.tl("budi@x.co").await;
    let created = |cookie: &str, name: &'static str, owner: &'static str| {
        let (app, cookie) = (&app, cookie.to_string());
        async move {
            let form = [("name", name), ("owner_email", owner)];
            app.post_form("/projects", &form, Some(&cookie)).await
        }
    };
    let res = created(&admin, "Payments", "rina@x.co").await;
    assert_eq!(res.status(), 303);
    assert_eq!(app.get(&location(&res), Some(&rina)).await.status(), 200);

    let res = created(&budi, "Shop", "rina@x.co").await;
    assert_eq!(res.status(), 303);
    let shop = location(&res);
    assert_eq!(app.get(&shop, Some(&budi)).await.status(), 200);
    assert_eq!(app.get(&shop, Some(&rina)).await.status(), 404);

    let res = created(&admin, "Ghost", "nobody@x.co").await;
    assert_eq!(res.status(), 422);
    assert!(
        res.text()
            .await
            .unwrap()
            .contains("Pick the owner from the list")
    );
}

const PNG_1X1: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==";

#[tokio::test]
async fn a_project_keeps_its_description_and_serves_its_icon_to_its_owner_only() {
    let app = TestApp::spawn("test").await;
    let rina = app.tl("rina@x.co").await;
    let budi = app.tl("budi@x.co").await;
    let form = [
        ("name", "Payments"),
        ("description", "  Card and bank payments for the shop.  "),
        ("icon", &format!("data:image/png;base64,{PNG_1X1}")),
    ];
    let res = app.post_form("/projects", &form, Some(&rina)).await;
    assert_eq!(res.status(), 303);
    let slug = location(&res).replace("/monitors", "");
    let overview = app.get(&slug, Some(&rina)).await.text().await.unwrap();
    assert!(overview.contains(">Card and bank payments for the shop.</p>"));
    assert!(overview.contains(&format!(r#"src="{slug}/icon?v="#)));

    let icon = app.get(&format!("{slug}/icon"), Some(&rina)).await;
    assert_eq!(icon.status(), 200);
    assert_eq!(icon.headers()["content-type"], "image/png");
    assert!(
        icon.headers()["cache-control"]
            .to_str()
            .unwrap()
            .contains("immutable")
    );
    assert_eq!(icon.bytes().await.unwrap().len(), 70);
    let other = app.get(&format!("{slug}/icon"), Some(&budi)).await;
    assert_eq!(other.status(), 404);
}

#[tokio::test]
async fn the_edit_dialog_refuses_svg_and_long_text_and_removes_the_icon() {
    let app = TestApp::spawn("test").await;
    let rina = app.tl("rina@x.co").await;
    project(&app, &rina, "Payments", "payments").await;
    let edit = |form: Vec<(&'static str, String)>| {
        let (app, rina) = (&app, rina.clone());
        async move {
            let form: Vec<(&str, &str)> = form.iter().map(|(k, v)| (*k, v.as_str())).collect();
            app.post_form("/projects/payments/profile", &form, Some(&rina))
                .await
        }
    };
    let svg = "data:image/svg+xml;base64,PHN2Zy8+".to_string();
    let res = edit(vec![("icon", svg)]).await;
    assert_eq!(res.status(), 422);
    let html = res.text().await.unwrap();
    assert!(html.contains("Use a PNG, JPEG or WebP picture.") && html.contains("data-open"));
    let res = edit(vec![("description", "x".repeat(281))]).await;
    assert_eq!(res.status(), 422);

    let res = edit(vec![
        ("icon", PNG_1X1.to_string()),
        ("description", "Shop".into()),
    ])
    .await;
    assert_eq!(res.status(), 303);
    assert_eq!(
        app.get("/projects/payments/icon", Some(&rina))
            .await
            .status(),
        200
    );
    let res = edit(vec![
        ("remove_icon", "on".into()),
        ("description", String::new()),
    ])
    .await;
    assert_eq!(res.status(), 303);
    assert_eq!(
        app.get("/projects/payments/icon", Some(&rina))
            .await
            .status(),
        404
    );
    let general = app
        .get("/projects/payments/settings", Some(&rina))
        .await
        .text()
        .await
        .unwrap();
    assert!(general.contains("None yet"));
}

#[tokio::test]
async fn the_project_search_matches_name_owner_and_description_within_the_users_projects() {
    let app = TestApp::spawn("test").await;
    let admin = app.super_admin("admin@x.co").await;
    let rina = app.tl("rina@x.co").await;
    let budi = app.tl("budi@x.co").await;
    for (cookie, name, description) in [
        (&rina, "Payments", "Card payments"),
        (&rina, "Warehouse", "Nightly stock sync"),
        (&budi, "Shop", "Storefront"),
    ] {
        let form = [("name", name), ("description", description)];
        app.post_form("/projects", &form, Some(cookie)).await;
    }
    let home = |cookie: &str, q: &'static str| {
        let (app, cookie) = (&app, cookie.to_string());
        async move {
            app.get(&format!("/?q={q}"), Some(&cookie))
                .await
                .text()
                .await
                .unwrap()
        }
    };
    let html = home(&rina, "stock").await;
    assert!(
        html.contains("<strong>Warehouse</strong>") && !html.contains("<strong>Payments</strong>")
    );
    let html = home(&admin, "BUDI@").await;
    assert!(html.contains("<strong>Shop</strong>") && !html.contains("<strong>Warehouse</strong>"));
    let html = home(&rina, "shop").await;
    assert!(
        html.contains("No project matches"),
        "a TL never finds another TL's project"
    );
}

#[tokio::test]
async fn a_tl_cannot_reassign() {
    let app = TestApp::spawn("test").await;
    let rina = app.tl("rina@x.co").await;
    project(&app, &rina, "Payments", "payments").await;
    let res = app
        .post_form(
            "/projects/payments/owner",
            &[("owner_email", "rina@x.co")],
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
    let keys_page = app
        .get("/projects/payments/settings/keys", Some(&tl))
        .await
        .text()
        .await
        .unwrap();
    assert!(keys_page.contains("backend-prod") && keys_page.contains(&key[..7]));
    assert!(!keys_page.contains(&key), "the full key appears only once");

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
    let res = app
        .post_form(
            &format!("/projects/payments/keys/{}/revoke", keys[0].id),
            &[],
            Some(&tl),
        )
        .await;
    assert_eq!(location(&res), "/projects/payments/settings/keys");
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
    let page = app
        .get("/projects/payments/settings/allowlist", Some(&tl))
        .await
        .text()
        .await
        .unwrap();
    assert!(page.contains("Allowed IP addresses</h2><span class=\"tag\">Off"));
    assert_eq!(ref_id(&app, Some(&key), Value::Null).await.status(), 201);

    let res = app
        .post_form(
            "/projects/payments/allowlist",
            &[("cidrs", "10.20.0.0/16")],
            Some(&tl),
        )
        .await;
    assert_eq!(location(&res), "/projects/payments/settings/allowlist");
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
