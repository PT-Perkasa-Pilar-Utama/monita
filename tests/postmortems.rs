//! Post-mortems (v1.2 Q63): every move of the state machine through the routes, the emails a
//! publish sends, who may write, the size cap, and that the text never reaches a public page.

mod common;

use common::{BASE_URL, TestApp, json_of};
use reqwest::Method;
use serde_json::{Value, json};

const PATH: &str = "/api/ui/projects/payments/incidents/1/postmortem";
const MARKER: &str = "PM-MARKER-7f3a";

async fn sql(app: &TestApp, sql: &str) {
    let sql = sql.to_string();
    app.state
        .db
        .write(async move |c| {
            c.execute_batch(&sql).await?;
            Ok(())
        })
        .await
        .unwrap();
}

async fn count(app: &TestApp, sql: &str) -> i64 {
    let c = app.state.db.read().await.unwrap();
    monita::platform::db::query_i64(&c, sql, ())
        .await
        .unwrap()
        .unwrap_or(0)
}

/// A project with a public monitor and one open automatic incident on it.
async fn setup(app: &TestApp, tl: &str) {
    app.project(tl, "Payments", "payments").await;
    sql(
        app,
        "INSERT INTO monitors (id, project_id, name, url, next_check_at, is_public) VALUES
           (1, 1, 'API', 'https://api.invalid', 9999999999, 1);
         INSERT INTO incidents (id, project_id, origin, title, severity, cause, started_at) VALUES
           (1, 1, 'auto', 'API is down', 'critical', 'HTTP 503', unixepoch() - 7200);
         INSERT INTO incident_items VALUES (1, 'monitor', 1);",
    )
    .await;
}

async fn put(app: &TestApp, cookie: &str, body: Value) -> reqwest::Response {
    app.api_send(Method::PUT, PATH, body, Some(cookie)).await
}

fn filled(marker: &str) -> Value {
    json!({
        "summary": format!("The API answered 503 for an hour. {marker}"),
        "impact": "Checkout failed for card payments.",
        "root_cause": "The edge cache filled its disk.",
        "trigger": "A spike in traffic.",
        "resolution": "The cache was flushed and its disk grown.",
        "detection": "Monita's monitor, after two failed checks.",
        "action_items": "Alert on cache disk use.",
        "lessons": "Size caches for peak traffic."
    })
}

fn with_action(action: &str, mut body: Value) -> Value {
    body["action"] = json!(action);
    body
}

#[tokio::test]
async fn a_post_mortem_moves_from_none_to_draft_to_published_and_back() {
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    setup(&app, &tl).await;

    let res = put(&app, &tl, json!({ "action": "save" })).await;
    assert_eq!(res.status(), 422);
    assert_eq!(
        json_of(res).await["error"]["message"],
        "Resolve the incident before you write its post-mortem."
    );
    sql(
        &app,
        "UPDATE incidents SET resolved_at = unixepoch() - 3600 WHERE id = 1",
    )
    .await;
    let view = json_of(app.api_get(PATH, Some(&tl)).await).await;
    assert_eq!(view["data"]["state"], "none");
    assert_eq!(view["data"]["recipients"], 0);

    // none -> draft, with nothing written yet.
    let res = put(
        &app,
        &tl,
        json!({ "action": "save", "summary": " First notes " }),
    )
    .await;
    assert_eq!(json_of(res).await["data"]["state"], "draft");
    let view = json_of(app.api_get(PATH, Some(&tl)).await).await;
    assert_eq!(view["data"]["summary"], "First notes");
    assert_eq!(view["data"]["updated_by"], "rina@x.co");

    // A publish needs summary, root cause and resolution.
    let res = put(&app, &tl, json!({ "action": "publish", "summary": "x" })).await;
    assert_eq!(res.status(), 422);
    let fields = json_of(res).await["error"]["fields"].clone();
    assert!(fields.get("root_cause").is_some() && fields.get("resolution").is_some());
    assert!(fields.get("summary").is_none());

    // draft -> published, emailing the list.
    let list = json!({ "enabled": true, "emails": "boss@x.co\nops@x.co" });
    let res = app
        .api_send(
            Method::PUT,
            "/api/ui/projects/payments/postmortem-recipients",
            list,
            Some(&tl),
        )
        .await;
    assert_eq!(res.status(), 204);
    let res = put(&app, &tl, with_action("publish", filled(MARKER))).await;
    let out = json_of(res).await["data"].clone();
    assert_eq!(
        (out["state"].clone(), out["emailed"].clone()),
        (json!("published"), json!(2))
    );
    assert_eq!(
        count(&app, "SELECT count(*) FROM outbox WHERE subject = 'Post-mortem: INC-1 API is down' AND attachment IS NOT NULL").await,
        2
    );

    // Refused from published: save, publish again, delete.
    for body in [
        json!({ "action": "save" }),
        with_action("publish", filled(MARKER)),
    ] {
        assert_eq!(put(&app, &tl, body).await.status(), 422);
    }
    assert_eq!(
        app.api_send(Method::DELETE, PATH, json!({}), Some(&tl))
            .await
            .status(),
        422
    );

    // published -> draft -> published again: an update.
    let edit = format!("{PATH}/edit");
    let res = app
        .api_send(Method::POST, &edit, json!({}), Some(&tl))
        .await;
    assert_eq!(json_of(res).await["data"]["state"], "draft");
    assert_eq!(
        app.api_send(Method::POST, &edit, json!({}), Some(&tl))
            .await
            .status(),
        422,
        "already a draft"
    );
    let res = put(&app, &tl, with_action("publish", filled(MARKER))).await;
    assert_eq!(json_of(res).await["data"]["emailed"], 2);
    assert_eq!(
        count(
            &app,
            "SELECT count(*) FROM outbox WHERE subject = 'Post-mortem updated: INC-1 API is down'"
        )
        .await,
        2
    );

    // A draft that was published once is not deleted.
    app.api_send(Method::POST, &edit, json!({}), Some(&tl))
        .await;
    assert_eq!(
        app.api_send(Method::DELETE, PATH, json!({}), Some(&tl))
            .await
            .status(),
        422
    );

    // draft -> none, for one never published.
    sql(&app, "DELETE FROM postmortems").await;
    put(&app, &tl, json!({ "action": "save", "summary": "notes" })).await;
    assert_eq!(
        app.api_send(Method::DELETE, PATH, json!({}), Some(&tl))
            .await
            .status(),
        204
    );
    let view = json_of(app.api_get(PATH, Some(&tl)).await).await;
    assert_eq!(view["data"]["state"], "none");
    assert_eq!(
        app.api_send(Method::DELETE, PATH, json!({}), Some(&tl))
            .await
            .status(),
        422
    );
}

#[tokio::test]
async fn the_incident_pdf_carries_the_post_mortem_marked_draft_until_published() {
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    setup(&app, &tl).await;
    sql(
        &app,
        "UPDATE incidents SET resolved_at = unixepoch() WHERE id = 1",
    )
    .await;
    put(&app, &tl, with_action("save", filled("draft-words"))).await;
    let pdf = app
        .get("/api/ui/projects/payments/incidents/1/report", Some(&tl))
        .await
        .bytes()
        .await
        .unwrap();
    let text = String::from_utf8_lossy(&inflate(&pdf)).into_owned();
    assert!(text.contains(r"(Post-mortem \(draft\))"));
    assert!(text.contains("(Root cause)") && text.contains("(The edge cache filled its disk.)"));
    put(&app, &tl, with_action("publish", filled("x"))).await;
    let pdf = app
        .get("/api/ui/projects/payments/incidents/1/report", Some(&tl))
        .await
        .bytes()
        .await
        .unwrap();
    let text = String::from_utf8_lossy(&inflate(&pdf)).into_owned();
    assert!(text.contains("(Post-mortem)") && !text.contains("draft)"));
}

#[tokio::test]
async fn only_the_owner_or_a_super_admin_writes_and_the_text_never_reaches_a_public_page() {
    let app = TestApp::spawn("test").await;
    let admin = app.super_admin("boss@x.co").await;
    let tl = app.tl("rina@x.co").await;
    let budi = app.tl("budi@x.co").await;
    setup(&app, &tl).await;
    sql(
        &app,
        "UPDATE incidents SET resolved_at = unixepoch() WHERE id = 1",
    )
    .await;
    assert_eq!(
        put(&app, &budi, json!({ "action": "save" })).await.status(),
        404
    );
    assert_eq!(app.api_get(PATH, Some(&budi)).await.status(), 404);
    let res = put(&app, &admin, with_action("publish", filled(MARKER))).await;
    assert_eq!(res.status(), 200);

    let res = app
        .api_post(
            "/api/ui/admin/users",
            json!({ "email": "sari@x.co", "role": "supervisor" }),
            Some(&admin),
        )
        .await;
    let link = json_of(res).await["data"]["link"]
        .as_str()
        .unwrap()
        .to_string();
    let sup = app
        .accept(link.strip_prefix(BASE_URL).unwrap(), common::PASSWORD)
        .await;
    assert_eq!(app.api_get(PATH, Some(&sup)).await.status(), 200);
    assert_eq!(
        app.api_send(Method::POST, &format!("{PATH}/edit"), json!({}), Some(&sup))
            .await
            .status(),
        403
    );

    for path in [
        "/api/v1/public/status/payments",
        "/api/ui/public/payments/status",
        "/api/ui/public/payments/history",
        "/api/ui/public/payments/incidents",
    ] {
        let res = app.get(path, None).await;
        assert_eq!(res.status(), 200, "{path}");
        let body = res.text().await.unwrap();
        assert!(
            body.contains("API is down") || path.ends_with("status"),
            "{path} shows the incident"
        );
        assert!(!body.contains(MARKER), "{path} leaks the post-mortem");
    }
}

#[tokio::test]
async fn the_largest_post_mortem_fits_the_body_cap_and_one_character_more_is_refused() {
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    setup(&app, &tl).await;
    sql(
        &app,
        "UPDATE incidents SET resolved_at = unixepoch() WHERE id = 1",
    )
    .await;
    // A control character takes 6 bytes in JSON (\u0001), the most a character can take.
    let max = "\u{1}".repeat(2_000);
    let fields = [
        "summary",
        "impact",
        "root_cause",
        "trigger",
        "resolution",
        "detection",
        "action_items",
        "lessons",
    ];
    let mut body = json!({ "action": "save" });
    for f in fields {
        body[f] = json!(format!("a{max}").chars().take(2_000).collect::<String>());
    }
    assert!(serde_json::to_vec(&body).unwrap().len() > 90_000);
    assert_eq!(put(&app, &tl, body.clone()).await.status(), 200);
    body["lessons"] = json!("x".repeat(2_001));
    let res = put(&app, &tl, body).await;
    assert_eq!(res.status(), 422);
    assert!(json_of(res).await["error"]["fields"]["lessons"].is_string());
}

/// Every content stream of a PDF, inflated.
fn inflate(pdf: &[u8]) -> Vec<u8> {
    use std::io::Read as _;
    let find = |hay: &[u8], n: &[u8]| hay.windows(n.len()).position(|w| w == n);
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(i) = find(&pdf[at..], b">>\nstream\n") {
        let start = at + i + 10;
        let end = start + find(&pdf[start..], b"\nendstream").unwrap();
        flate2::read::ZlibDecoder::new(&pdf[start..end])
            .read_to_end(&mut out)
            .unwrap();
        at = end + 10;
    }
    out
}
