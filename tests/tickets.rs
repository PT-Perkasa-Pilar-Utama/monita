//! US-24 to US-30. AC: docs/business/acceptance-criteria-breakdown/acceptance-criteria-sprint-4.md.

mod common;

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::{Router, extract::State, routing::post};
use common::{TestApp, json_of};
use monita::platform::{db::query_i64, now, settings};
use serde_json::{Value, json};

struct Setup {
    tl: String,
    key: String,
}

async fn setup(app: &TestApp) -> Setup {
    let tl = app.tl("rina@x.co").await;
    app.project(&tl, "Payments", "payments").await;
    let key = monita::projects::create_key(&app.state.db, 1, "backend")
        .await
        .unwrap();
    Setup { tl, key }
}

async fn ref_id(app: &TestApp, key: &str, single_use: bool) -> String {
    let res = app
        .http
        .post(app.url("/api/v1/ref-ids"))
        .bearer_auth(key)
        .json(&json!({"single_use": single_use}))
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    body["data"]["ref_id"].as_str().unwrap().to_string()
}

async fn submit(app: &TestApp, ref_id: &str, title: &str) -> reqwest::Response {
    app.http
        .post(app.url("/api/v1/tickets"))
        .json(&json!({"ref_id": ref_id, "title": title, "description": "Card 4111 failed at the last step", "page_url": "https://shop.example/pay"}))
        .send()
        .await
        .unwrap()
}

async fn count(app: &TestApp, sql: &str) -> i64 {
    query_i64(&app.state.db.read().await.unwrap(), sql, ())
        .await
        .unwrap()
        .unwrap_or(0)
}

const PUBLIC_BOARD: &str = "/api/ui/public/payments/tickets";

/// The `data` of a JSON read.
async fn read(app: &TestApp, path: &str, cookie: Option<&str>) -> Value {
    json_of(app.api_get(path, cookie).await).await["data"].take()
}

/// Moves ticket 1, as the board's Move ticket button does.
async fn stage(app: &TestApp, tl: &str, body: Value) -> reqwest::Response {
    let path = "/api/ui/projects/payments/tickets/1/stage";
    app.api_send(reqwest::Method::PUT, path, body, Some(tl))
        .await
}

async fn revoke(app: &TestApp, tl: &str) -> reqwest::Response {
    let path = "/api/ui/projects/payments/tickets/1/revoke-ref";
    app.api_post(path, json!({}), Some(tl)).await
}

#[tokio::test]
async fn a_reporter_submits_through_the_api_and_the_form() {
    // AC-24.01, AC-24.02, AC-25.02
    let app = TestApp::spawn("test").await;
    let s = setup(&app).await;
    let rid = ref_id(&app, &s.key, false).await;
    let res = submit(&app, &rid, "Checkout button does nothing").await;
    assert_eq!(res.status(), 201);
    assert_eq!(res.headers()["access-control-allow-origin"], "*");
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["data"]["number"], 1);
    assert_eq!(body["data"]["stage"], "triage");
    let ticket = read(&app, "/api/ui/projects/payments/tickets/1", Some(&s.tl)).await;
    assert_eq!(ticket["title"], "Checkout button does nothing");
    assert_eq!(ticket["description"], "Card 4111 failed at the last step");
    assert_eq!(ticket["page_url"], "https://shop.example/pay");
    assert_eq!(ticket["stage"], "triage");
    assert_eq!(ticket["ref_revoked"], false);
    assert_eq!(
        ticket["ref_tail"].as_str(),
        Some(&rid[rid.len() - 4..]),
        "the team tells ref IDs apart by their last 4 characters"
    );
    assert!(!ticket.to_string().contains(&rid), "never the whole ref ID");

    let form = app
        .api_post(
            "/api/ui/public/payments/report",
            json!({ "ref_id": rid, "title": "Receipt shows wrong currency", "description": "IDR shown as USD", "page_url": "" }),
            None,
        )
        .await;
    assert_eq!(form.status(), 201);
    assert_eq!(json_of(form).await["data"]["number"], 2);
    assert_eq!(
        count(&app, "SELECT count(*) FROM tickets WHERE stage = 'triage'").await,
        2
    );
}

#[tokio::test]
async fn bad_ref_ids_are_refused_before_the_caps_are_counted() {
    // AC-24.03, AC-27.01
    let app = TestApp::spawn("test").await;
    let s = setup(&app).await;
    let unknown = submit(&app, "rf_nosuchid", "A real title").await;
    assert_eq!(unknown.status(), 422);
    let body: Value = unknown.json().await.unwrap();
    assert_eq!(body["error"]["code"], "ref_id_invalid");

    let once = ref_id(&app, &s.key, true).await;
    assert_eq!(submit(&app, &once, "First and only").await.status(), 201);
    assert_eq!(
        submit(&app, &once, "Second try").await.status(),
        422,
        "single-use is used up"
    );

    let other_tl = app.tl("budi@x.co").await;
    app.project(&other_tl, "Other", "other").await;
    let rid = ref_id(&app, &s.key, false).await;
    let wrong_project = app
        .api_post(
            "/api/ui/public/other/report",
            json!({ "ref_id": rid, "title": "Wrong project", "description": "x" }),
            None,
        )
        .await;
    assert_eq!(
        wrong_project.status(),
        422,
        "a ref ID works only on its own project's form"
    );
    assert_eq!(
        json_of(wrong_project).await["error"]["fields"]["ref_id"],
        "This ref ID is unknown, revoked or already used."
    );

    let res = revoke(&app, &s.tl).await;
    assert_eq!(res.status(), 204);
    assert_eq!(
        count(&app, "SELECT count(*) FROM tickets").await,
        1,
        "AC-27.02: tickets stay"
    );
}

#[tokio::test]
async fn daily_caps_per_ref_id_and_per_project() {
    // AC-24.04, AC-24.05
    let app = TestApp::spawn("test").await;
    let s = setup(&app).await;
    app.state
        .db
        .write(async |c| settings::set(c, "tickets_per_ref_per_day", 2).await)
        .await
        .unwrap();
    app.state
        .db
        .write(async |c| settings::set(c, "tickets_per_project_per_day", 3).await)
        .await
        .unwrap();
    let a = ref_id(&app, &s.key, false).await;
    assert_eq!(
        submit(&app, &a, "Ok").await.status(),
        400,
        "too short title"
    );
    assert_eq!(submit(&app, &a, "First").await.status(), 201);
    assert_eq!(submit(&app, &a, "Second").await.status(), 201);
    let capped = submit(&app, &a, "Third").await;
    assert_eq!(capped.status(), 429);
    let body: Value = capped.json().await.unwrap();
    assert_eq!(body["error"]["code"], "daily_cap_reached");
    let b = ref_id(&app, &s.key, false).await;
    assert_eq!(submit(&app, &b, "Fourth").await.status(), 201);
    assert_eq!(submit(&app, &b, "Fifth").await.status(), 429, "project cap");
}

#[tokio::test]
async fn kanban_moves_need_a_reason_for_done_and_leaving_done_clears_it() {
    // AC-25.01, AC-25.03
    let app = TestApp::spawn("test").await;
    let s = setup(&app).await;
    let rid = ref_id(&app, &s.key, false).await;
    submit(&app, &rid, "Checkout broken").await;
    let board = read(&app, "/api/ui/projects/payments/tickets", Some(&s.tl)).await;
    assert_eq!(
        board["counts"],
        json!({ "triage": 1, "backlog": 0, "in_progress": 0, "done": 0 })
    );
    let no_reason = stage(&app, &s.tl, json!({ "stage": "done", "close_reason": "" })).await;
    assert_eq!(no_reason.status(), 422);
    assert_eq!(
        json_of(no_reason).await["error"]["fields"]["close_reason"],
        "Pick a close reason."
    );
    let done = stage(
        &app,
        &s.tl,
        json!({ "stage": "done", "close_reason": "wont_fix" }),
    )
    .await;
    assert_eq!(done.status(), 204);
    stage(
        &app,
        &s.tl,
        json!({ "stage": "backlog", "close_reason": "wont_fix" }),
    )
    .await;
    assert_eq!(
        count(
            &app,
            "SELECT count(*) FROM tickets WHERE stage = 'backlog' AND close_reason IS NULL"
        )
        .await,
        1
    );
}

#[tokio::test]
async fn the_public_board_hides_triage_descriptions_and_reporters() {
    // AC-26.01, AC-28.01, AC-28.02
    let app = TestApp::spawn("test").await;
    let s = setup(&app).await;
    let rid = ref_id(&app, &s.key, false).await;
    submit(&app, &rid, "Card of Budi Santoso declined").await;
    submit(&app, &rid, "Still in triage").await;
    let res = app.api_get(PUBLIC_BOARD, None).await;
    assert_eq!(res.headers()["cache-control"], "public, max-age=30");
    let board = read(&app, PUBLIC_BOARD, None).await;
    assert!(board["counts"].get("triage").is_none(), "not even a count");
    let board = board.to_string();
    assert!(
        !board.contains("Budi Santoso") && !board.contains("Still in triage"),
        "Triage is private"
    );

    let res = app
        .api_send(
            reqwest::Method::PUT,
            "/api/ui/projects/payments/tickets/1/public-title",
            json!({ "public_title": "A card is declined at checkout" }),
            Some(&s.tl),
        )
        .await;
    assert_eq!(res.status(), 204);
    stage(&app, &s.tl, json!({ "stage": "in_progress" })).await;
    let board = read(&app, PUBLIC_BOARD, None).await.to_string();
    assert!(board.contains("A card is declined at checkout"));
    assert!(
        !board.contains("Budi Santoso"),
        "the original title stays private"
    );
    assert!(!board.contains("4111"), "descriptions never appear");
    assert!(!board.contains(&rid), "reporters never appear");
    stage(
        &app,
        &s.tl,
        json!({ "stage": "done", "close_reason": "wont_fix" }),
    )
    .await;
    let board = read(&app, PUBLIC_BOARD, None).await.to_string();
    assert!(board.contains("A card is declined at checkout"));
    assert!(
        !board.contains("wont_fix") && !board.contains("close_reason"),
        "the close reason is the TL's, not public"
    );
}

#[tokio::test]
async fn a_reporter_lists_their_tickets_even_after_revocation() {
    // AC-29.01, AC-27.02
    let app = TestApp::spawn("test").await;
    let s = setup(&app).await;
    let rid = ref_id(&app, &s.key, false).await;
    submit(&app, &rid, "Checkout broken").await;
    assert_eq!(revoke(&app, &s.tl).await.status(), 204);
    assert_eq!(
        revoke(&app, &s.tl).await.status(),
        204,
        "revoking twice is a no-op"
    );
    assert_eq!(submit(&app, &rid, "After revoke").await.status(), 422);
    let res = app
        .http
        .get(app.url("/api/v1/tickets/mine"))
        .header("x-ref-id", &rid)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["data"][0]["title"], "Checkout broken");
    assert_eq!(body["data"][0]["stage"], "triage");
    assert!(body["data"][0].get("description").is_none());
    assert_eq!(
        app.http
            .get(app.url("/api/v1/tickets/mine"))
            .send()
            .await
            .unwrap()
            .status(),
        401
    );

    let mine = app
        .api_post(
            "/api/ui/public/payments/mine",
            json!({ "ref_id": rid }),
            None,
        )
        .await;
    let mine = json_of(mine).await;
    assert_eq!(mine["data"]["tickets"][0]["title"], "Checkout broken");
    assert_eq!(mine["data"]["tickets"][0]["stage"], "triage");
    assert!(mine["data"]["tickets"][0].get("description").is_none());
}

#[tokio::test]
async fn browsers_get_a_cors_preflight() {
    let app = TestApp::spawn("test").await;
    for path in ["/api/v1/tickets", "/api/v1/tickets/mine"] {
        let res = app
            .http
            .request(reqwest::Method::OPTIONS, app.url(path))
            .header("origin", "https://shop.example")
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), 204, "{path}");
        assert_eq!(res.headers()["access-control-allow-origin"], "*");
        assert!(
            res.headers()["access-control-allow-headers"]
                .to_str()
                .unwrap()
                .contains("x-ref-id")
        );
    }
}

#[tokio::test]
async fn the_digest_lists_new_triage_tickets_once_and_skips_quiet_projects() {
    // AC-30.01, AC-30.02
    let app = TestApp::spawn("test").await;
    let s = setup(&app).await;
    let quiet = app.tl("budi@x.co").await;
    app.project(&quiet, "Quiet", "quiet").await;
    let rid = ref_id(&app, &s.key, false).await;
    submit(&app, &rid, "Checkout broken").await;
    submit(&app, &rid, "Receipt wrong").await;

    assert_eq!(
        monita::notify::digest(&app.state.db, common::BASE_URL, now())
            .await
            .unwrap(),
        1
    );
    monita::notify::deliver_due(&app.state).await.unwrap();
    let digests: Vec<_> = app
        .state
        .mailer
        .sent()
        .into_iter()
        .filter(|m| m.subject.starts_with("[Monita]"))
        .collect();
    assert_eq!(digests.len(), 1);
    assert_eq!(digests[0].to, "rina@x.co");
    assert_eq!(digests[0].subject, "[Monita] Payments: 2 new tickets");
    assert!(
        digests[0].body.contains("#1  Checkout broken")
            && digests[0].body.contains("/projects/payments/tickets/2")
    );
    assert!(
        !digests[0].body.contains("4111"),
        "no descriptions in email"
    );
    assert_eq!(
        monita::notify::digest(&app.state.db, common::BASE_URL, now() + 1)
            .await
            .unwrap(),
        0,
        "nothing new since"
    );
}

/// Webhook calls a test server got, by path name.
type Calls = Arc<Mutex<Vec<(String, Value)>>>;

#[tokio::test]
async fn each_new_ticket_calls_the_ticket_webhook_and_never_the_warning_one() {
    // AC-30.03
    let got: Calls = Arc::default();
    let sink = Router::new()
        .route(
            "/{name}",
            post(
                |State(g): State<Calls>,
                 axum::extract::Path(name): axum::extract::Path<String>,
                 body: String| async move {
                    g.lock()
                        .unwrap()
                        .push((name, serde_json::from_str(&body).unwrap()));
                    "ok"
                },
            ),
        )
        .with_state(got.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, sink).await.unwrap() });

    let app = TestApp::spawn("test").await;
    let s = setup(&app).await;
    let alerts = "/api/ui/projects/payments/ticket-alerts";
    assert_eq!(
        read(&app, alerts, Some(&s.tl)).await,
        json!({"enabled": true, "digest": true, "webhook_url": null, "use_webhook": false, "webhook_moves": false, "webhook_format": "json", "webhook_key": null}),
        "a new project: digest on, webhook off"
    );
    let put = |body: Value| app.api_send(reqwest::Method::PUT, alerts, body, Some(&s.tl));
    app.alert_card(
        &s.tl,
        "payments",
        json!({ "kind": "json", "name": "Warning hook", "warning": true, "url": format!("http://{addr}/warning") }),
    )
    .await;
    let rid = ref_id(&app, &s.key, false).await;
    submit(&app, &rid, "Off: no webhook").await;

    let res = put(json!({"digest": true, "use_webhook": true})).await;
    assert_eq!(res.status(), 422, "a webhook switched on needs its URL");
    let hook = format!("http://{addr}/tickets");
    assert_eq!(
        put(json!({"digest": true, "webhook_url": &hook, "use_webhook": true}))
            .await
            .status(),
        204
    );
    submit(&app, &rid, "On: webhook").await;
    let team = json!({"title": "Filed by the team", "description": "x", "stage": "backlog"});
    app.api_post("/api/ui/projects/payments/tickets", team, Some(&s.tl))
        .await;

    monita::notify::deliver_due(&app.state).await.unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;
    let calls = got.lock().unwrap().clone();
    assert_eq!(
        calls.len(),
        1,
        "one reporter ticket while on; the team's sends nothing"
    );
    assert_eq!(calls[0].0, "tickets", "the warning webhook gets no tickets");
    assert_eq!(calls[0].1["event"], "ticket.created");
    assert_eq!(calls[0].1["ticket_number"], 2);
    assert_eq!(calls[0].1["text"], "[TICKET] Payments #2: On: webhook");
}

#[tokio::test]
async fn a_column_move_calls_the_ticket_webhook_only_when_moves_are_on() {
    let app = TestApp::spawn("test").await;
    let s = setup(&app).await;
    let rid = ref_id(&app, &s.key, false).await;
    submit(&app, &rid, "Checkout broken").await;
    let alerts = "/api/ui/projects/payments/ticket-alerts";
    let put = |body: Value| app.api_send(reqwest::Method::PUT, alerts, body, Some(&s.tl));
    let hooks = "SELECT count(*) FROM outbox WHERE channel = 'webhook'";

    let res = put(json!({"digest": true, "webhook_moves": true})).await;
    assert_eq!(res.status(), 422, "moves need the webhook URL");
    let url = "http://127.0.0.1:9/t";
    let off = json!({"digest": true, "webhook_url": url, "webhook_moves": false});
    assert_eq!(put(off).await.status(), 204);
    stage(&app, &s.tl, json!({ "stage": "backlog" })).await;
    assert_eq!(
        count(&app, hooks).await,
        0,
        "moves off: the URL alone sends nothing"
    );

    let on = json!({"digest": true, "webhook_url": url, "webhook_moves": true});
    assert_eq!(put(on).await.status(), 204);
    stage(&app, &s.tl, json!({ "stage": "backlog" })).await;
    assert_eq!(count(&app, hooks).await, 0, "the same column is no move");
    let done = json!({ "stage": "done", "close_reason": "fixed" });
    assert_eq!(stage(&app, &s.tl, done).await.status(), 204);
    submit(&app, &rid, "New tickets stay off").await;
    assert_eq!(count(&app, hooks).await, 1, "one move; new tickets are off");

    let c = app.state.db.read().await.unwrap();
    let body: String = monita::platform::db::query_all(
        &c,
        "SELECT body FROM outbox WHERE channel = 'webhook'",
        (),
        |r| r.get(0),
    )
    .await
    .unwrap()
    .remove(0);
    let body: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["event"], "ticket.moved");
    assert_eq!(
        body["text"],
        "[TICKET] Payments #1 moved from Backlog to Done (Fixed): Checkout broken"
    );
    assert_eq!(
        (&body["from"], &body["to"], &body["close_reason"]),
        (&json!("backlog"), &json!("done"), &json!("fixed"))
    );

    let test = "/api/ui/projects/payments/ticket-alerts/test";
    let res = app.api_post(test, json!({}), Some(&s.tl)).await;
    assert_eq!(
        json_of(res).await["data"]["sent_to"],
        1,
        "the test button works with moves alone"
    );
}

#[tokio::test]
async fn the_digest_skips_a_project_that_turned_it_off() {
    let app = TestApp::spawn("test").await;
    let s = setup(&app).await;
    let rid = ref_id(&app, &s.key, false).await;
    submit(&app, &rid, "Checkout broken").await;
    let off = json!({"digest": false, "webhook_url": "", "use_webhook": false});
    let res = app
        .api_send(
            reqwest::Method::PUT,
            "/api/ui/projects/payments/ticket-alerts",
            off,
            Some(&s.tl),
        )
        .await;
    assert_eq!(res.status(), 204);
    let queued = monita::notify::digest(&app.state.db, common::BASE_URL, now())
        .await
        .unwrap();
    assert_eq!(queued, 0);
}

#[tokio::test]
async fn a_column_shows_25_and_pages_the_rest_on_both_boards() {
    let app = TestApp::spawn("test").await;
    let s = setup(&app).await;
    app.state
        .db
        .write(async |c| {
            c.execute(
                "INSERT INTO ref_ids (id, project_id, token_hash) VALUES (1, 1, 'h')",
                (),
            )
            .await?;
            for n in 1..=61 {
                let (stage, reason) = if n <= 31 { ("done", Some("fixed")) } else { ("backlog", None) };
                c.execute(
                    "INSERT INTO tickets (project_id, number, ref_id, title, description, stage, close_reason, updated_at) VALUES (1, ?1, 1, ?2, 'd', ?3, ?4, ?1)",
                    (n, format!("Ticket {n}"), stage, reason),
                )
                .await?;
            }
            Ok(())
        })
        .await
        .unwrap();
    let titles = |v: &Value| -> Vec<String> {
        v["tickets"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["title"].as_str().unwrap().to_string())
            .collect()
    };
    for (path, cookie) in [
        ("/api/ui/projects/payments/tickets", Some(s.tl.as_str())),
        (PUBLIC_BOARD, None),
    ] {
        let board = read(&app, path, cookie).await;
        let first = titles(&board);
        assert_eq!(first.len(), 50, "{path}: 25 a column");
        assert!(
            !first.contains(&"Ticket 6".to_string()),
            "{path}: the 26th Done waits"
        );
        assert!(first.contains(&"Ticket 7".to_string()));
        assert_eq!(
            board["counts"]["done"], 31,
            "{path}: the count is every ticket"
        );

        let more = read(&app, &format!("{path}?stage=done&page=2"), cookie).await;
        assert_eq!(
            titles(&more),
            [
                "Ticket 6", "Ticket 5", "Ticket 4", "Ticket 3", "Ticket 2", "Ticket 1"
            ],
            "{path}: page 2 holds the rest, newest first"
        );
        let past = read(&app, &format!("{path}?stage=done&page=3"), cookie).await;
        assert!(titles(&past).is_empty(), "{path}: past the end is empty");
    }
    let res = app
        .api_get(
            "/api/ui/projects/payments/tickets?stage=archived",
            Some(&s.tl),
        )
        .await;
    assert_eq!(res.status(), 404, "an unknown column");
}

#[tokio::test]
async fn the_public_column_pages_never_show_triage() {
    let app = TestApp::spawn("test").await;
    let s = setup(&app).await;
    let r = ref_id(&app, &s.key, false).await;
    assert_eq!(submit(&app, &r, "Private report").await.status(), 201);
    let page = app.get(&format!("{PUBLIC_BOARD}?stage=triage"), None).await;
    assert_eq!(page.status(), 404, "Triage is not a public column");
}

#[tokio::test]
async fn the_team_files_a_ticket_without_a_ref_id() {
    let app = TestApp::spawn("test").await;
    let s = setup(&app).await;
    let path = "/api/ui/projects/payments/tickets";
    let body = json!({"title": "Refund button is grey", "description": "Seen in the admin panel.", "stage": "backlog"});
    let res = app.api_post(path, body.clone(), Some(&s.tl)).await;
    assert_eq!(res.status(), 201);
    assert_eq!(json_of(res).await["data"]["number"], 1);

    let t = read(&app, "/api/ui/projects/payments/tickets/1", Some(&s.tl)).await;
    assert_eq!(t["stage"], "backlog");
    assert_eq!(t["filed_by"], "rina@x.co");
    assert_eq!(t["ref_revoked"], false);
    assert_eq!(
        count(
            &app,
            "SELECT count(*) FROM tickets WHERE ref_id IS NULL AND created_by IS NOT NULL"
        )
        .await,
        1
    );
    assert_eq!(
        count(&app, "SELECT count(*) FROM ref_ids").await,
        0,
        "no ref ID is made up"
    );

    let done = json!({"title": "Refund button is grey", "description": "x", "stage": "done"});
    let res = app.api_post(path, done, Some(&s.tl)).await;
    assert_eq!(res.status(), 422);
    assert!(json_of(res).await["error"]["fields"]["stage"].is_string());

    let other = app.tl("budi@x.co").await;
    assert_eq!(
        app.api_post(path, body, Some(&other)).await.status(),
        404,
        "only the owner files"
    );
}

#[tokio::test]
async fn the_digest_skips_a_disabled_owner() {
    let app = TestApp::spawn("test").await;
    let s = setup(&app).await;
    let rid = ref_id(&app, &s.key, false).await;
    submit(&app, &rid, "Checkout broken").await;
    app.super_admin("admin@x.co").await;
    let users = monita::accounts::list_users(&app.state.db).await.unwrap();
    let id_of = |email: &str| users.iter().find(|u| u.email == email).unwrap().id;
    monita::accounts::disable(&app.state.db, id_of("admin@x.co"), id_of("rina@x.co"))
        .await
        .unwrap();
    assert_eq!(
        monita::notify::digest(&app.state.db, common::BASE_URL, now())
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn a_public_tab_turned_off_answers_404_and_the_app_api_still_files() {
    let app = TestApp::spawn("test").await;
    let s = setup(&app).await;
    let res = app
        .api_send(
            reqwest::Method::PUT,
            "/api/ui/projects/payments/public-pages",
            json!({ "known_issues": true, "report": false }),
            Some(&s.tl),
        )
        .await;
    assert_eq!(res.status(), 204);
    let project = read(&app, "/api/ui/public/payments", None).await;
    assert_eq!(
        project["pages"],
        json!({ "incidents": true, "known_issues": true, "report": false, "changelog": true }),
        "a client that leaves out incidents or changelog reads it as on"
    );

    let rid = ref_id(&app, &s.key, false).await;
    let form = app
        .api_post(
            "/api/ui/public/payments/report",
            json!({ "ref_id": rid, "title": "Receipt shows wrong currency", "description": "", "page_url": "" }),
            None,
        )
        .await;
    assert_eq!(form.status(), 404, "the status page form is off");
    assert_eq!(
        submit(&app, &rid, "Checkout button does nothing")
            .await
            .status(),
        201,
        "the project's own app still files through the API"
    );
    assert_eq!(app.api_get(PUBLIC_BOARD, None).await.status(), 200);
    let mine = app
        .api_post(
            "/api/ui/public/payments/mine",
            json!({ "ref_id": rid }),
            None,
        )
        .await;
    assert_eq!(
        mine.status(),
        404,
        "a reporter's own tickets live in the Report a bug tab, so they go with it"
    );
}

#[tokio::test]
async fn the_team_issues_a_ref_id_from_the_board_and_it_files_tickets() {
    let app = TestApp::spawn("test").await;
    let s = setup(&app).await;
    let path = "/api/ui/projects/payments/ref-ids";
    let other = app.tl("budi@x.co").await;
    assert_eq!(
        app.api_post(path, json!({}), Some(&other)).await.status(),
        404,
        "only the owner"
    );

    let res = app
        .api_post(path, json!({ "single_use": true }), Some(&s.tl))
        .await;
    assert_eq!(res.status(), 201);
    let rid = json_of(res).await["data"]["ref_id"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(rid.starts_with("rf_"), "{rid}");
    assert_eq!(submit(&app, &rid, "Reported by phone").await.status(), 201);
    assert_eq!(
        submit(&app, &rid, "A second report").await.status(),
        422,
        "single use"
    );
    let stored = format!("SELECT count(*) FROM ref_ids WHERE token_hash = '{rid}'");
    assert_eq!(count(&app, &stored).await, 0, "only the hash is stored");
}

#[tokio::test]
async fn a_report_form_names_every_bad_field_at_once() {
    let app = TestApp::spawn("test").await;
    setup(&app).await;
    let res = app
        .api_post(
            "/api/ui/public/payments/report",
            json!({ "ref_id": "", "title": "x", "description": "", "page_url": "ftp://x" }),
            None,
        )
        .await;
    assert_eq!(res.status(), 422);
    let fields = json_of(res).await["error"]["fields"].clone();
    for f in ["ref_id", "title", "description", "page_url"] {
        assert!(fields[f].is_string(), "{f} missing from {fields}");
    }
}
