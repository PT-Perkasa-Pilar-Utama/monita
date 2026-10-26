//! US-05 to US-08, and the ref-id API that the key rules guard (AC-23.01 to AC-23.03).

mod common;

use common::{TestApp, json_of};
use serde_json::{Value, json};

async fn project(app: &TestApp, cookie: &str, name: &str, slug: &str) -> reqwest::StatusCode {
    app.project(cookie, name, slug).await
}

/// Creates a project through the API and returns its slug.
async fn create(app: &TestApp, cookie: &str, mut body: Value) -> String {
    body.as_object_mut()
        .unwrap()
        .entry("homepage_url")
        .or_insert("https://app.example.com".into());
    let res = app.api_post("/api/ui/projects", body, Some(cookie)).await;
    assert_eq!(res.status(), 201);
    json_of(res).await["data"]["slug"]
        .as_str()
        .unwrap()
        .to_string()
}

/// The slugs on a user's project list.
async fn listed(app: &TestApp, cookie: &str) -> Vec<String> {
    let list = json_of(app.get("/api/ui/projects", Some(cookie)).await).await;
    list["data"]["projects"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["slug"].as_str().unwrap().to_string())
        .collect()
}

/// Creates a key through the API and returns the full key, which the answer carries once.
async fn new_key(app: &TestApp, cookie: &str, slug: &str) -> String {
    let res = app
        .api_post(
            &format!("/api/ui/projects/{slug}/keys"),
            json!({ "name": "backend-prod" }),
            Some(cookie),
        )
        .await;
    assert_eq!(res.status(), 201);
    json_of(res).await["data"]["key"]
        .as_str()
        .unwrap()
        .to_string()
}

/// A PUT from the frontend.
async fn put(app: &TestApp, path: &str, body: Value, cookie: &str) -> reqwest::Response {
    app.api_send(reqwest::Method::PUT, path, body, Some(cookie))
        .await
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
    assert_eq!(project(&app, &tl, "Payments", "payments").await, 201);
    let list = json_of(app.get("/api/ui/projects", Some(&tl)).await).await;
    let p = &list["data"]["projects"][0];
    assert_eq!(
        (p["slug"].as_str(), p["owner_email"].as_str()),
        (Some("payments"), Some("rina@x.co"))
    );
}

#[tokio::test]
async fn monita_makes_a_unique_slug_from_the_name() {
    // AC-06.02
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    let mut slugs = Vec::new();
    for _ in 0..2 {
        let slug = create(&app, &tl, json!({ "name": "Customer Portal" })).await;
        let (base, suffix) = slug.rsplit_once('-').unwrap();
        assert_eq!(base, "customer-portal");
        assert_eq!(suffix.len(), 6);
        let path = format!("/api/ui/public/{slug}/status");
        let status = app.get(&path, None).await.status();
        assert_eq!(status, 200, "AC-06.03");
        slugs.push(slug);
    }
    assert_ne!(slugs[0], slugs[1]);
    let blank = app
        .api_post("/api/ui/projects", json!({ "name": "  " }), Some(&tl))
        .await;
    assert_eq!(blank.status(), 422);
    let fields = json_of(blank).await["error"]["fields"].clone();
    assert_eq!(fields["name"], "Enter a name of up to 80 characters.");
    assert!(
        fields["homepage_url"].is_string(),
        "every bad field at once: {fields}"
    );
}

#[tokio::test]
async fn the_overview_sums_up_the_project_and_its_head_carries_the_tab_counts() {
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    project(&app, &tl, "Payments", "payments").await;
    new_key(&app, &tl, "payments").await;
    let o = json_of(
        app.get("/api/ui/projects/payments/overview", Some(&tl))
            .await,
    )
    .await;
    let o = &o["data"];
    assert_eq!(o["monitors"]["total"], 0);
    assert_eq!(
        (o["triage"].as_i64(), o["active_keys"].as_i64()),
        (Some(0), Some(1))
    );
    assert_eq!(o["incidents"].as_array().unwrap().len(), 0);
    let head = json_of(app.get("/api/ui/projects/payments", Some(&tl)).await).await;
    assert_eq!(head["data"]["project"]["owner_email"], "rina@x.co");
    assert_eq!(head["data"]["counts"]["monitors"], 0);
}

#[tokio::test]
async fn another_tl_cannot_see_the_project() {
    let app = TestApp::spawn("test").await;
    let rina = app.tl("rina@x.co").await;
    let budi = app.tl("budi@x.co").await;
    project(&app, &rina, "Payments", "payments").await;
    for path in [
        "/api/ui/projects/payments",
        "/api/ui/projects/payments/overview",
        "/api/ui/projects/payments/keys",
        "/api/ui/projects/payments/alert-cards",
        "/projects/payments/icon",
    ] {
        assert_eq!(app.get(path, Some(&budi)).await.status(), 404, "{path}");
    }
    let write = put(
        &app,
        "/api/ui/projects/payments/allowlist",
        json!({ "cidrs": "" }),
        &budi,
    )
    .await;
    assert_eq!(write.status(), 404, "a write is refused the same way");
    assert!(listed(&app, &budi).await.is_empty());
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

    let res = put(
        &app,
        "/api/ui/projects/payments/owner",
        json!({ "owner_email": " Budi@X.co " }),
        &admin,
    )
    .await;
    assert_eq!(res.status(), 204);
    assert_eq!(listed(&app, &budi).await, ["payments"]);
    assert!(listed(&app, &rina).await.is_empty());
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
    let users = monita::accounts::list_users(&app.state.db).await.unwrap();
    let id_of = |email: &str| users.iter().find(|u| u.email == email).unwrap().id;
    monita::accounts::disable(&app.state.db, id_of("admin@x.co"), id_of("budi@x.co"))
        .await
        .unwrap();
    for owner in ["budi@x.co", "nobody@x.co", ""] {
        let body = json!({ "owner_email": owner });
        let res = put(&app, "/api/ui/projects/payments/owner", body, &admin).await;
        assert_eq!(res.status(), 422, "owner {owner}");
        let err = json_of(res).await;
        assert!(
            err["error"]["fields"]["owner_email"]
                .as_str()
                .unwrap()
                .starts_with("Pick the owner from the list")
        );
    }
    assert_eq!(
        listed(&app, &rina).await,
        ["payments"],
        "rina still owns it"
    );
}

#[tokio::test]
async fn the_owner_list_offers_the_admin_first_then_active_tls_and_only_to_a_super_admin() {
    let app = TestApp::spawn("test").await;
    let admin = app.super_admin("admin@x.co").await;
    let rina = app.tl("rina@x.co").await;
    project(&app, &rina, "Payments", "payments").await;
    let list = json_of(
        app.get("/api/ui/projects/payments/owners", Some(&admin))
            .await,
    )
    .await;
    let emails: Vec<&str> = list["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| o["email"].as_str().unwrap())
        .collect();
    assert_eq!(emails, ["admin@x.co", "rina@x.co"]);
    let tl = app
        .get("/api/ui/projects/payments/owners", Some(&rina))
        .await;
    assert_eq!(tl.status(), 403);
}

#[tokio::test]
async fn a_super_admin_names_the_owner_and_a_tl_always_owns_what_they_create() {
    let app = TestApp::spawn("test").await;
    let admin = app.super_admin("admin@x.co").await;
    let rina = app.tl("rina@x.co").await;
    let budi = app.tl("budi@x.co").await;
    let payments = create(
        &app,
        &admin,
        json!({ "name": "Payments", "owner_email": "rina@x.co" }),
    )
    .await;
    assert_eq!(listed(&app, &rina).await, [payments]);

    let shop = create(
        &app,
        &budi,
        json!({ "name": "Shop", "owner_email": "rina@x.co" }),
    )
    .await;
    assert_eq!(
        listed(&app, &budi).await,
        [shop],
        "a TL owns what they create"
    );
    assert_eq!(listed(&app, &rina).await.len(), 1);

    let ghost = json!({ "name": "Ghost", "owner_email": "nobody@x.co", "homepage_url": "https://ghost.example.com" });
    let res = app.api_post("/api/ui/projects", ghost, Some(&admin)).await;
    assert_eq!(res.status(), 422);
    assert!(
        json_of(res).await["error"]["fields"]["owner_email"]
            .as_str()
            .unwrap()
            .starts_with("Pick the owner from the list")
    );
}

const PNG_1X1: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==";

#[tokio::test]
async fn a_project_keeps_its_description_and_serves_its_icon_to_its_owner_only() {
    let app = TestApp::spawn("test").await;
    let rina = app.tl("rina@x.co").await;
    let budi = app.tl("budi@x.co").await;
    let body = json!({
        "name": "Payments",
        "description": "  Card and bank payments for the shop.  ",
        "icon": format!("data:image/png;base64,{PNG_1X1}"),
    });
    let slug = format!("/projects/{}", create(&app, &rina, body).await);
    let list = json_of(app.get("/api/ui/projects", Some(&rina)).await).await;
    let p = &list["data"]["projects"][0];
    assert_eq!(p["description"], "Card and bank payments for the shop.");
    assert!(
        p["icon_url"]
            .as_str()
            .unwrap()
            .starts_with(&format!("{slug}/icon?v="))
    );

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
async fn editing_refuses_svg_and_long_text_and_removes_the_icon() {
    let app = TestApp::spawn("test").await;
    let rina = app.tl("rina@x.co").await;
    project(&app, &rina, "Payments", "payments").await;
    let edit = |mut body: Value| {
        body["homepage_url"] = "https://pay.example.com".into();
        put(&app, "/api/ui/projects/payments/profile", body, &rina)
    };
    let res = edit(json!({ "icon": "data:image/svg+xml;base64,PHN2Zy8+" })).await;
    assert_eq!(res.status(), 422);
    assert_eq!(
        json_of(res).await["error"]["fields"]["icon"],
        "Use a PNG, JPEG or WebP picture."
    );
    let res = edit(json!({ "description": "x".repeat(281) })).await;
    assert_eq!(res.status(), 422);

    let res = edit(json!({ "icon": PNG_1X1, "description": "Shop" })).await;
    assert_eq!(res.status(), 204);
    let icon = app.get("/projects/payments/icon", Some(&rina)).await;
    assert_eq!(icon.status(), 200);
    let res = edit(json!({ "remove_icon": "on", "description": "" })).await;
    assert_eq!(res.status(), 204);
    let icon = app.get("/projects/payments/icon", Some(&rina)).await;
    assert_eq!(icon.status(), 404);
    let head = json_of(app.get("/api/ui/projects/payments", Some(&rina)).await).await;
    assert!(head["data"]["project"]["description"].is_null());
    assert!(head["data"]["project"]["icon_url"].is_null());
}

#[tokio::test]
async fn the_project_list_holds_only_the_users_projects_and_all_for_a_super_admin() {
    let app = TestApp::spawn("test").await;
    let admin = app.super_admin("admin@x.co").await;
    let rina = app.tl("rina@x.co").await;
    let budi = app.tl("budi@x.co").await;
    for (cookie, name) in [(&rina, "Payments"), (&rina, "Warehouse"), (&budi, "Shop")] {
        create(&app, cookie, json!({ "name": name })).await;
    }
    assert_eq!(listed(&app, &rina).await.len(), 2);
    assert_eq!(listed(&app, &budi).await.len(), 1);
    assert_eq!(listed(&app, &admin).await.len(), 3);
}

/// One page of the project list as (slugs, total, page).
async fn page_of(app: &TestApp, cookie: &str, query: &str) -> (Vec<String>, i64, i64) {
    let res = app
        .get(&format!("/api/ui/projects{query}"), Some(cookie))
        .await;
    assert_eq!(res.status(), 200);
    let d = json_of(res).await["data"].clone();
    assert_eq!(d["page_size"], 20);
    let slugs = d["projects"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["name"].as_str().unwrap().to_string())
        .collect();
    (
        slugs,
        d["total"].as_i64().unwrap(),
        d["page"].as_i64().unwrap(),
    )
}

#[tokio::test]
async fn the_project_list_pages_20_at_a_time_newest_first() {
    let app = TestApp::spawn("test").await;
    let admin = app.super_admin("admin@x.co").await;
    let rina = app.tl("rina@x.co").await;
    let budi = app.tl("budi@x.co").await;
    for i in 1..=22 {
        create(&app, &rina, json!({ "name": format!("Svc {i:02}") })).await;
    }
    create(&app, &budi, json!({ "name": "Shop" })).await;

    let (first, total, page) = page_of(&app, &rina, "").await;
    assert_eq!((first.len(), total, page), (20, 22, 1));
    assert_eq!(first[0], "Svc 22");
    assert_eq!(
        page_of(&app, &rina, "?page=2").await,
        (vec!["Svc 02".into(), "Svc 01".into()], 22, 2)
    );
    assert_eq!(page_of(&app, &rina, "?page=0").await.0, first);
    assert_eq!(page_of(&app, &rina, "?page=abc").await.0, first);
    assert_eq!(page_of(&app, &rina, "?page=99").await, (vec![], 22, 99));
    assert_eq!(page_of(&app, &admin, "").await.1, 23);
}

#[tokio::test]
async fn the_project_list_searches_every_page_on_the_server() {
    let app = TestApp::spawn("test").await;
    let admin = app.super_admin("admin@x.co").await;
    let rina = app.tl("rina@x.co").await;
    let budi = app.tl("budi@x.co").await;
    for i in 1..=22 {
        create(&app, &rina, json!({ "name": format!("Svc {i:02}") })).await;
    }
    create(
        &app,
        &budi,
        json!({ "name": "Shop", "description": "Handles refunds" }),
    )
    .await;
    create(&app, &budi, json!({ "name": "Svc Budi" })).await;

    let (names, total, _) = page_of(&app, &rina, "?q=%20SVC%202%20").await;
    assert_eq!(
        (names, total),
        (vec!["Svc 22".into(), "Svc 21".into(), "Svc 20".into()], 3)
    );
    assert_eq!(page_of(&app, &admin, "?q=budi%40").await.1, 2);
    assert_eq!(page_of(&app, &admin, "?q=refund").await.0, ["Shop"]);
    assert_eq!(page_of(&app, &rina, "?q=budi").await.1, 0);
    assert_eq!(page_of(&app, &admin, "?q=%25").await.1, 0);
}

#[tokio::test]
async fn the_project_list_filters_by_owner_and_sorts_by_age() {
    let app = TestApp::spawn("test").await;
    let admin = app.super_admin("admin@x.co").await;
    let rina = app.tl("rina@x.co").await;
    let budi = app.tl("budi@x.co").await;
    for (cookie, name) in [(&rina, "Beta"), (&budi, "Alpha"), (&rina, "Gamma")] {
        create(&app, cookie, json!({ "name": name })).await;
    }
    sql_ages(&app).await;

    let rows = page_of(&app, &admin, "?owner=rina%40x.co").await;
    assert_eq!((rows.0, rows.1), (vec!["Gamma".into(), "Beta".into()], 2));
    assert_eq!(
        page_of(&app, &admin, "?sort=name").await.0,
        ["Alpha", "Beta", "Gamma"]
    );
    assert_eq!(
        page_of(&app, &admin, "?sort=oldest").await.0,
        ["Beta", "Alpha", "Gamma"]
    );
    assert_eq!(
        page_of(&app, &admin, "?sort=junk").await.0,
        ["Gamma", "Alpha", "Beta"],
        "anything else is newest first"
    );
    let list = json_of(app.get("/api/ui/projects", Some(&admin)).await).await;
    assert_eq!(
        list["data"]["owner_options"],
        json!(["budi@x.co", "rina@x.co"])
    );
    let list = json_of(app.get("/api/ui/projects", Some(&rina)).await).await;
    assert_eq!(list["data"]["owner_options"], json!([]));
}

/// Spreads the projects' creation times: Beta oldest, then Alpha, then Gamma.
async fn sql_ages(app: &TestApp) {
    app.state
        .db
        .write(async |c| {
            c.execute_batch(
                "UPDATE projects SET created_at = 100 WHERE name = 'Beta';
                 UPDATE projects SET created_at = 200 WHERE name = 'Alpha';
                 UPDATE projects SET created_at = 300 WHERE name = 'Gamma';",
            )
            .await?;
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn a_tl_cannot_reassign() {
    let app = TestApp::spawn("test").await;
    let rina = app.tl("rina@x.co").await;
    project(&app, &rina, "Payments", "payments").await;
    let body = json!({ "owner_email": "rina@x.co" });
    let res = put(&app, "/api/ui/projects/payments/owner", body, &rina).await;
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
    let keys = app
        .get("/api/ui/projects/payments/keys", Some(&tl))
        .await
        .text()
        .await
        .unwrap();
    assert!(keys.contains("backend-prod") && keys.contains(&key[..9]));
    assert!(!keys.contains(&key), "the full key appears only once");

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
        .api_post(
            &format!("/api/ui/projects/payments/keys/{}/revoke", keys[0].id),
            json!({}),
            Some(&tl),
        )
        .await;
    assert_eq!(res.status(), 204);
    let revoked = ref_id(&app, Some(&key), Value::Null).await;
    assert_eq!(revoked.status(), 401);
    let err: Value = revoked.json().await.unwrap();
    assert_eq!(err["error"]["code"], "unauthorized");
}

#[tokio::test]
async fn a_key_works_on_a_project_with_a_description_and_marks_only_itself_used() {
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    let slug = create(
        &app,
        &tl,
        json!({ "name": "Reports", "description": "Monthly reports." }),
    )
    .await;
    new_key(&app, &tl, &slug).await;
    let second = new_key(&app, &tl, &slug).await;
    assert_eq!(ref_id(&app, Some(&second), json!({})).await.status(), 201);
    let keys = json_of(
        app.get(&format!("/api/ui/projects/{slug}/keys"), Some(&tl))
            .await,
    )
    .await;
    let used: Vec<i64> = keys["data"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|k| !k["last_used_at"].is_null())
        .map(|k| k["id"].as_i64().unwrap())
        .collect();
    assert_eq!(used, [2], "{keys}");
}

/// Rows in `table` that belong to the deleted project's data.
async fn rows(app: &TestApp, sql: &str) -> i64 {
    monita::platform::db::query_i64(&app.state.db.read().await.unwrap(), sql, ())
        .await
        .unwrap()
        .unwrap_or(0)
}

#[tokio::test]
async fn a_super_admin_deletes_a_project_and_everything_in_it() {
    let app = TestApp::spawn("test").await;
    let admin = app.super_admin("admin@x.co").await;
    let rina = app.tl("rina@x.co").await;
    let doomed = create(&app, &rina, json!({ "name": "Doomed" })).await;
    let kept = create(&app, &rina, json!({ "name": "Kept" })).await;
    new_key(&app, &rina, &doomed).await;
    app.state
        .db
        .write(async |c| {
            c.execute_batch(
                "INSERT INTO monitors (id, project_id, name, url, next_check_at) VALUES (7, 1, 'm', 'https://x.co', 9999999999);
                 INSERT INTO heartbeats (id, project_id, name, token, period_s, grace_s) VALUES (8, 1, 'h', 'tok', 60, 0);
                 INSERT INTO daily_summaries (source_kind, source_id, day) VALUES ('monitor', 7, '2026-10-01'), ('heartbeat', 8, '2026-10-01');
                 INSERT INTO incident_sources (source_kind, source_id, fail_streak) VALUES ('monitor', 7, 1);
                 INSERT INTO ref_ids (project_id, token_hash) VALUES (1, 'ref');
                 INSERT INTO tickets (project_id, number, ref_id, title, description) VALUES (1, 1, 1, 'bug', 'd');",
            )
            .await?;
            Ok(())
        })
        .await
        .unwrap();
    let path = format!("/api/ui/projects/{doomed}");

    let tl = app
        .api_send(
            reqwest::Method::DELETE,
            &path,
            json!({ "confirm": "Doomed" }),
            Some(&rina),
        )
        .await;
    assert_ne!(tl.status(), 204);
    let wrong = app
        .api_send(
            reqwest::Method::DELETE,
            &path,
            json!({ "confirm": "doomed" }),
            Some(&admin),
        )
        .await;
    assert_eq!(wrong.status(), 422);
    assert!(json_of(wrong).await["error"]["fields"]["confirm"].is_string());
    assert_eq!(rows(&app, "SELECT count(*) FROM projects").await, 2);

    let ok = app
        .api_send(
            reqwest::Method::DELETE,
            &path,
            json!({ "confirm": "Doomed" }),
            Some(&admin),
        )
        .await;
    assert_eq!(ok.status(), 204);
    assert_eq!(app.get(&path, Some(&admin)).await.status(), 404);
    assert_eq!(listed(&app, &rina).await, [kept]);
    for table in [
        "monitors",
        "heartbeats",
        "daily_summaries",
        "incident_sources",
        "tickets",
        "ref_ids",
        "api_keys",
        "alert_cards",
    ] {
        let left = rows(
            &app,
            &format!(
                "SELECT count(*) FROM {table} WHERE {}",
                if table == "alert_cards" {
                    "project_id = 1"
                } else {
                    "1"
                }
            ),
        )
        .await;
        assert_eq!(left, 0, "{table}");
    }
}

#[tokio::test]
async fn the_allowlist_is_off_by_default_and_blocks_other_ips() {
    // AC-08.01, AC-08.02
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    project(&app, &tl, "Payments", "payments").await;
    let key = new_key(&app, &tl, "payments").await;
    let head = json_of(app.get("/api/ui/projects/payments", Some(&tl)).await).await;
    assert!(
        head["data"]["project"]["ip_allowlist"].is_null(),
        "off by default"
    );
    assert_eq!(ref_id(&app, Some(&key), Value::Null).await.status(), 201);

    let allow = |cidrs: &'static str| {
        put(
            &app,
            "/api/ui/projects/payments/allowlist",
            json!({ "cidrs": cidrs }),
            &tl,
        )
    };
    assert_eq!(allow("10.20.0.0/16").await.status(), 204);
    let blocked = ref_id(&app, Some(&key), Value::Null).await;
    assert_eq!(
        blocked.status(),
        403,
        "the test client calls from 127.0.0.1"
    );

    allow("10.20.0.0/16\n127.0.0.0/8").await;
    assert_eq!(ref_id(&app, Some(&key), Value::Null).await.status(), 201);

    let bad = allow("intranet").await;
    assert_eq!(bad.status(), 422);
    assert!(json_of(bad).await["error"]["fields"]["cidrs"].is_string());
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

#[tokio::test]
async fn the_project_list_shows_each_projects_state_incidents_tickets_and_last_day() {
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    project(&app, &tl, "Payments", "payments").await;
    let now = monita::platform::now();
    app.state
        .db
        .write(async |c| {
            c.execute_batch(&format!(
                "INSERT INTO monitors (id, project_id, name, url, next_check_at, created_at) VALUES (1, 1, 'API', 'https://x.invalid', 9999999999, {old});
                 INSERT INTO incidents (id, project_id, origin, title, severity, started_at) VALUES (1, 1, 'auto', 'API is down', 'critical', {start});
                 INSERT INTO incident_items VALUES (1, 'monitor', 1);
                 INSERT INTO incident_sources VALUES ('monitor', 1, 2, 1, NULL);",
                start = now - 7200,
                old = now - 2 * 86_400,
            ))
            .await?;
            Ok(())
        })
        .await
        .unwrap();
    let list = json_of(app.api_get("/api/ui/projects", Some(&tl)).await).await;
    let pulse = &list["data"]["projects"][0]["pulse"];
    assert_eq!(pulse["kind"], "down");
    assert_eq!(pulse["incidents"], 1);
    assert_eq!(pulse["monitors"]["total"], 1);
    let hours = pulse["hours"].as_array().unwrap();
    assert_eq!(hours.len(), 24);
    assert_eq!(hours[0]["uptime"], 100.0, "a day ago, before the incident");
    assert_eq!(hours[22]["uptime"], 0.0, "the one system was down all hour");

    project(&app, &tl, "Fresh", "fresh").await;
    app.state
        .db
        .write(async |c| {
            c.execute(
                "INSERT INTO monitors (project_id, name, url, next_check_at, created_at) VALUES (2, 'New', 'https://x.invalid', 9999999999, unixepoch() - 60)",
                (),
            )
            .await?;
            Ok(())
        })
        .await
        .unwrap();
    let list = json_of(app.api_get("/api/ui/projects", Some(&tl)).await).await;
    let fresh = list["data"]["projects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["slug"] == "fresh")
        .unwrap();
    let hours = fresh["pulse"]["hours"].as_array().unwrap();
    assert!(hours[0]["uptime"].is_null(), "no data before it was added");
    assert_eq!(hours[23]["uptime"], 100.0);
}

#[tokio::test]
async fn a_new_project_needs_a_homepage_that_its_status_page_links_to() {
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    let bad = app
        .api_post(
            "/api/ui/projects",
            json!({ "name": "Shop", "homepage_url": "shop.example.com" }),
            Some(&tl),
        )
        .await;
    assert_eq!(bad.status(), 422);
    assert!(json_of(bad).await["error"]["fields"]["homepage_url"].is_string());
    let slug = create(
        &app,
        &tl,
        json!({ "name": "Shop", "homepage_url": " https://shop.example.com " }),
    )
    .await;
    let site = json_of(app.get(&format!("/api/ui/public/{slug}"), None).await).await;
    assert_eq!(site["data"]["homepage_url"], "https://shop.example.com");
}

#[tokio::test]
async fn the_tab_numbers_flag_only_what_needs_a_look() {
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    project(&app, &tl, "Payments", "payments").await;
    app.state
        .db
        .write(async |c| {
            c.execute_batch(
                "INSERT INTO monitors (id, project_id, name, url, next_check_at) VALUES
                   (1, 1, 'API', 'https://a.invalid', 9999999999), (2, 1, 'Site', 'https://b.invalid', 9999999999);
                 INSERT INTO incidents (id, project_id, origin, title, severity, started_at) VALUES (1, 1, 'auto', 'API is down', 'critical', 1);
                 INSERT INTO incident_items VALUES (1, 'monitor', 1);
                 INSERT INTO incident_sources VALUES ('monitor', 1, 2, 1, NULL);
                 INSERT INTO tickets (project_id, number, created_by, stage, title, description, created_at, updated_at) VALUES
                   (1, 1, (SELECT min(id) FROM users), 'triage', 'New one', 'd', 1, 1), (1, 2, (SELECT min(id) FROM users), 'backlog', 'Sorted', 'd', 2, 2);",
            )
            .await?;
            Ok(())
        })
        .await
        .unwrap();
    let head = json_of(app.get("/api/ui/projects/payments", Some(&tl)).await).await;
    let c = &head["data"]["counts"];
    assert_eq!(
        (&c["monitors"], &c["monitors_down"]),
        (&json!(2), &json!(1)),
        "{c}"
    );
    assert_eq!(
        (&c["incidents"], &c["tickets"], &c["triage"]),
        (&json!(1), &json!(2), &json!(1))
    );
}

/// The homepage is public on the status page, so it carries no user name or password.
#[tokio::test]
async fn a_homepage_with_a_password_in_it_is_refused() {
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    let res = app
        .api_post(
            "/api/ui/projects",
            json!({ "name": "Shop", "homepage_url": "https://bob:secret@shop.example.com" }),
            Some(&tl),
        )
        .await;
    assert_eq!(res.status(), 422);
    let err = json_of(res).await;
    assert!(
        err["error"]["fields"]["homepage_url"]
            .as_str()
            .unwrap()
            .contains("no user name or password")
    );
}

/// v1.1 Q43: the list filters by the state its cards show.
#[tokio::test]
async fn the_project_list_filters_by_state_as_the_cards_show_it() {
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    for (name, slug) in [
        ("Down", "down"),
        ("Issue", "issue"),
        ("Quiet", "quiet"),
        ("Fine", "fine"),
    ] {
        project(&app, &tl, name, slug).await;
    }
    app.state
        .db
        .write(async |c| {
            c.execute_batch(
                "INSERT INTO monitors (id, project_id, name, url, next_check_at) VALUES
                   (1, 1, 'A', 'https://x.invalid', 9999999999), (2, 2, 'B', 'https://x.invalid', 9999999999),
                   (3, 3, 'C', 'https://x.invalid', 9999999999), (4, 4, 'D', 'https://x.invalid', 9999999999);
                 INSERT INTO incidents (id, project_id, origin, title, severity, started_at) VALUES
                   (1, 1, 'auto', 'A is down', 'critical', 1), (2, 2, 'manual', 'B is slow', 'warning', 1),
                   (3, 2, 'manual', 'not on a source', 'warning', 1);
                 INSERT INTO incident_items VALUES (1, 'monitor', 1), (2, 'monitor', 2);
                 INSERT INTO incident_sources (source_kind, source_id, fail_streak, open_incident_id, down_since)
                   VALUES ('monitor', 3, 3, NULL, 1);",
            )
            .await?;
            Ok(())
        })
        .await
        .unwrap();
    let slugs = async |state: &str| {
        let list = json_of(
            app.api_get(
                &format!("/api/ui/projects?state={state}&sort=name"),
                Some(&tl),
            )
            .await,
        )
        .await;
        list["data"]["projects"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p["slug"].as_str().unwrap().to_string())
            .collect::<Vec<_>>()
    };
    assert_eq!(
        slugs("down").await,
        ["down", "quiet"],
        "an incident switch off still reads down"
    );
    assert_eq!(slugs("issue").await, ["issue"]);
    assert_eq!(slugs("up").await, ["fine"]);
    assert_eq!(slugs("").await.len(), 4);
    let list = json_of(app.api_get("/api/ui/projects?state=down", Some(&tl)).await).await;
    assert_eq!(list["data"]["total"], 2, "the count follows the filter");
    let kinds: Vec<String> = json_of(app.api_get("/api/ui/projects?sort=name", Some(&tl)).await)
        .await["data"]["projects"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            format!(
                "{} {}",
                p["slug"].as_str().unwrap(),
                p["pulse"]["kind"].as_str().unwrap()
            )
        })
        .collect();
    assert_eq!(
        kinds,
        ["down down", "fine unknown", "issue issue", "quiet down"],
        "the badges agree with the filter"
    );
}

/// v1.1 Q39: the list's chart draws a line a project, from its monitors' checks, for the same
/// page and filter as the list.
#[tokio::test]
async fn the_project_chart_draws_a_line_a_project_for_the_page_and_filter() {
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    project(&app, &tl, "Shop", "shop").await;
    project(&app, &tl, "Blog", "blog").await;
    app.state
        .db
        .write(async |c| {
            c.execute_batch(
                "INSERT INTO monitors (id, project_id, name, url, next_check_at) VALUES
                   (1, 1, 'Cart', 'https://x.invalid', 9999999999), (2, 1, 'Pay', 'https://x.invalid', 9999999999);
                 INSERT INTO check_results (monitor_id, checked_at, ok, latency_ms) VALUES
                   (1, unixepoch() - 60, 1, 100), (2, unixepoch() - 60, 1, 300), (2, unixepoch() - 30, 0, 300);
                 INSERT INTO incidents (id, project_id, origin, title, severity, started_at) VALUES
                   (1, 1, 'auto', 'Pay is down', 'critical', unixepoch() - 600);
                 INSERT INTO incident_items VALUES (1, 'monitor', 2);
                 INSERT INTO heartbeats (id, project_id, name, token, period_s, grace_s, last_ping_at, deadline_at) VALUES
                   (1, 2, 'Backup', 'tok-b', 3600, 300, unixepoch() - 60, unixepoch() + 3600);
                 INSERT INTO pings (heartbeat_id, received_at, kind) VALUES (1, unixepoch() - 60, 'ok');",
            )
            .await?;
            Ok(())
        })
        .await
        .unwrap();
    let got = json_of(
        app.api_get("/api/ui/projects-chart?sort=name", Some(&tl))
            .await,
    )
    .await;
    let lines = got["data"]["chart"]["lines"].as_array().unwrap();
    assert_eq!(lines[0]["name"], "Shop", "down first");
    assert_eq!(lines[0]["state"], "down");
    assert_eq!(
        (&lines[0]["checks"], &lines[0]["failed"]),
        (&json!(3), &json!(1))
    );
    assert_eq!(
        lines[0]["mean_ms"], 233,
        "weighted by checks: (100 + 300 + 300) / 3"
    );
    let lanes = got["data"]["chart"]["lanes"].as_array().unwrap();
    assert_eq!(lanes.len(), 1, "a lane for the project with heartbeats");
    assert_eq!(
        (&lanes[0]["name"], &lanes[0]["id"]),
        (&json!("Blog"), &json!(2))
    );
    assert_eq!(
        got["data"]["chart"]["down"].as_array().unwrap().len(),
        1,
        "monitor down time shades the plot"
    );
    assert_eq!(got["data"]["slugs"]["1"], "shop");
    let up = json_of(
        app.api_get("/api/ui/projects-chart?state=up", Some(&tl))
            .await,
    )
    .await;
    let names: Vec<&str> = up["data"]["chart"]["lines"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["Blog"], "the filter applies");
}
