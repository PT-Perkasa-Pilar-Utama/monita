//! US-24 to US-30. AC: docs/business/acceptance-criteria-breakdown/acceptance-criteria-sprint-4.md.

mod common;

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::{Router, extract::State, routing::post};
use common::TestApp;
use monita::platform::{db::query_i64, now, settings};
use serde_json::{Value, json};

struct Setup {
    tl: String,
    key: String,
}

async fn setup(app: &TestApp) -> Setup {
    let tl = app.tl("rina@x.co").await;
    app.post_form(
        "/projects",
        &[("name", "Payments"), ("slug", "payments")],
        Some(&tl),
    )
    .await;
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

    let form = app
        .post_form(
            "/status/payments/report",
            &[
                ("ref_id", &rid),
                ("title", "Receipt shows wrong currency"),
                ("description", "IDR shown as USD"),
                ("page_url", ""),
            ],
            None,
        )
        .await;
    assert_eq!(form.status(), 200);
    assert!(
        form.text()
            .await
            .unwrap()
            .contains("Your report is ticket #2")
    );
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
    app.post_form(
        "/projects",
        &[("name", "Other"), ("slug", "other")],
        Some(&other_tl),
    )
    .await;
    let rid = ref_id(&app, &s.key, false).await;
    let wrong_project = app
        .post_form(
            "/status/other/report",
            &[
                ("ref_id", &rid),
                ("title", "Wrong project"),
                ("description", "x"),
            ],
            None,
        )
        .await;
    assert_eq!(
        wrong_project.status(),
        422,
        "a ref ID works only on its own project's form"
    );

    let res = app
        .post_form("/projects/payments/tickets/1/revoke-ref", &[], Some(&s.tl))
        .await;
    assert_eq!(res.status(), 303);
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
    let html = app
        .get("/projects/payments/tickets", Some(&s.tl))
        .await
        .text()
        .await
        .unwrap();
    for col in ["Triage", "Backlog", "In progress", "Done"] {
        assert!(html.contains(col), "missing column {col}");
    }
    let no_reason = app
        .post_form(
            "/projects/payments/tickets/1/stage",
            &[("stage", "done"), ("close_reason", "")],
            Some(&s.tl),
        )
        .await;
    assert_eq!(no_reason.status(), 422);
    assert!(
        no_reason
            .text()
            .await
            .unwrap()
            .contains("Pick a close reason.")
    );
    let done = app
        .post_form(
            "/projects/payments/tickets/1/stage",
            &[("stage", "done"), ("close_reason", "wont_fix")],
            Some(&s.tl),
        )
        .await;
    assert_eq!(done.status(), 303);
    app.post_form(
        "/projects/payments/tickets/1/stage",
        &[("stage", "backlog"), ("close_reason", "wont_fix")],
        Some(&s.tl),
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
    let board = app
        .get("/status/payments/tickets", None)
        .await
        .text()
        .await
        .unwrap();
    assert!(
        !board.contains("Budi Santoso") && !board.contains("Still in triage"),
        "Triage is private"
    );

    app.post_form(
        "/projects/payments/tickets/1/public-title",
        &[("public_title", "A card is declined at checkout")],
        Some(&s.tl),
    )
    .await;
    app.post_form(
        "/projects/payments/tickets/1/stage",
        &[("stage", "in_progress")],
        Some(&s.tl),
    )
    .await;
    let board = app
        .get("/status/payments/tickets", None)
        .await
        .text()
        .await
        .unwrap();
    assert!(board.contains("A card is declined at checkout"));
    assert!(
        !board.contains("Budi Santoso"),
        "the original title stays private"
    );
    assert!(!board.contains("4111"), "descriptions never appear");
    assert!(!board.contains(&rid), "reporters never appear");
}

#[tokio::test]
async fn a_reporter_lists_their_tickets_even_after_revocation() {
    // AC-29.01, AC-27.02
    let app = TestApp::spawn("test").await;
    let s = setup(&app).await;
    let rid = ref_id(&app, &s.key, false).await;
    submit(&app, &rid, "Checkout broken").await;
    app.post_form("/projects/payments/tickets/1/revoke-ref", &[], Some(&s.tl))
        .await;
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

    let page = app
        .post_form("/status/payments/mine", &[("ref_id", &rid)], None)
        .await
        .text()
        .await
        .unwrap();
    assert!(page.contains("Checkout broken") && page.contains("Triage"));
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
    app.post_form(
        "/projects",
        &[("name", "Quiet"), ("slug", "quiet")],
        Some(&quiet),
    )
    .await;
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

#[tokio::test]
async fn each_new_ticket_calls_the_warning_webhook_when_it_is_on() {
    // AC-30.03
    let got: Arc<Mutex<Vec<Value>>> = Arc::default();
    let sink = Router::new()
        .route(
            "/hook",
            post(
                |State(g): State<Arc<Mutex<Vec<Value>>>>, body: String| async move {
                    g.lock().unwrap().push(serde_json::from_str(&body).unwrap());
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
    let hook = format!("http://{addr}/hook");
    app.post_form(
        "/projects/payments/alerts/warning",
        &[("webhook_url", &hook)],
        Some(&s.tl),
    )
    .await;
    let rid = ref_id(&app, &s.key, false).await;
    submit(&app, &rid, "Off: no webhook").await;
    app.post_form(
        "/projects/payments/alerts/warning",
        &[("webhook_url", &hook), ("use_webhook", "on")],
        Some(&s.tl),
    )
    .await;
    submit(&app, &rid, "On: webhook").await;
    monita::notify::deliver_due(&app.state).await.unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;
    let calls = got.lock().unwrap().clone();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0]["event"], "ticket.created");
    assert_eq!(calls[0]["ticket_number"], 2);
    assert_eq!(calls[0]["text"], "[TICKET] Payments #2: On: webhook");
}

#[tokio::test]
async fn boards_show_every_open_ticket_and_the_newest_200_done() {
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
            for n in 1..=301 {
                let (stage, reason) = if n <= 201 { ("done", Some("fixed")) } else { ("backlog", None) };
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
    for (path, cookie) in [
        ("/projects/payments/tickets", Some(s.tl.as_str())),
        ("/status/payments/tickets", None),
    ] {
        let html = app.get(path, cookie).await.text().await.unwrap();
        assert_eq!(html.matches("class=\"card\"").count(), 300, "{path}");
        assert!(
            !html.contains(">Ticket 1<"),
            "{path}: the oldest Done drops off"
        );
        assert!(html.contains(">Ticket 2<") && html.contains(">Ticket 301<"));
        assert!(html.contains("Done shows the newest 200"));
    }
}

#[tokio::test]
async fn the_digest_skips_a_disabled_owner() {
    let app = TestApp::spawn("test").await;
    let s = setup(&app).await;
    let rid = ref_id(&app, &s.key, false).await;
    submit(&app, &rid, "Checkout broken").await;
    let rina = monita::accounts::list_users(&app.state.db)
        .await
        .unwrap()
        .into_iter()
        .find(|u| u.email == "rina@x.co")
        .unwrap()
        .id;
    monita::accounts::disable(&app.state.db, rina)
        .await
        .unwrap();
    assert_eq!(
        monita::notify::digest(&app.state.db, common::BASE_URL, now())
            .await
            .unwrap(),
        0
    );
}
