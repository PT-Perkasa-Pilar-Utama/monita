//! US-20: a manual incident's form errors come in the form's order.

mod common;

use common::TestApp;
use monita::checks::{self, MonitorInput};
use serde_json::json;

async fn setup(app: &TestApp) -> String {
    let tl = app.tl("rina@x.co").await;
    let status = app.project(&tl, "Payments", "payments").await;
    assert_eq!(status, 201, "the project exists");
    let m = MonitorInput {
        name: "API".into(),
        url: "https://x.invalid/".into(),
        ..MonitorInput::defaults()
    };
    checks::create_monitor(&app.state.db, 1, &m).await.unwrap();
    tl
}

/// Posts the create body the form sends: each pair a field, `items` collected into a list.
async fn error_for(app: &TestApp, tl: &str, form: &[(&str, &str)]) -> String {
    let mut body = serde_json::Map::new();
    for (k, v) in form.iter().filter(|(k, _)| *k != "items") {
        body.insert(k.to_string(), json!(v));
    }
    let items: Vec<&str> = form
        .iter()
        .filter(|(k, _)| *k == "items")
        .map(|(_, v)| *v)
        .collect();
    body.insert("items".into(), json!(items));
    let res = app
        .api_post("/api/ui/projects/payments/incidents", body.into(), Some(tl))
        .await;
    assert_eq!(res.status(), 422);
    res.text().await.unwrap()
}

#[tokio::test]
async fn title_then_items_then_severity_then_message() {
    let app = TestApp::spawn("test").await;
    let tl = setup(&app).await;
    let no_title = error_for(&app, &tl, &[("severity", "bogus")]).await;
    assert!(no_title.contains("Enter a title."));
    let no_items = error_for(&app, &tl, &[("title", "Slow"), ("severity", "bogus")]).await;
    assert!(no_items.contains("Pick at least one monitor or heartbeat."));
    let bad_severity = error_for(
        &app,
        &tl,
        &[
            ("title", "Slow"),
            ("items", "monitor:1"),
            ("severity", "bogus"),
        ],
    )
    .await;
    assert!(bad_severity.contains("Pick critical or warning."));
    let no_message = error_for(
        &app,
        &tl,
        &[
            ("title", "Slow"),
            ("items", "monitor:1"),
            ("severity", "warning"),
        ],
    )
    .await;
    assert!(no_message.contains("Write a message."));
}

#[tokio::test]
async fn the_list_shows_open_resolved_or_all_incidents() {
    let app = TestApp::spawn("test").await;
    let tl = setup(&app).await;
    for title in ["Slow sign-in", "Slow search"] {
        let body = json!({ "title": title, "items": ["monitor:1"], "severity": "warning", "message": "Looking into it." });
        let res = app
            .api_post("/api/ui/projects/payments/incidents", body, Some(&tl))
            .await;
        assert!(res.status().is_success(), "{}", res.status());
    }
    let res = app
        .api_post(
            "/api/ui/projects/payments/incidents/1/updates",
            json!({ "label": "resolved", "message": "Fixed." }),
            Some(&tl),
        )
        .await;
    assert!(res.status().is_success(), "{}", res.status());
    for (query, titles) in [
        ("", vec!["Slow search"]),
        ("?state=resolved", vec!["Slow sign-in"]),
        ("?state=all", vec!["Slow search", "Slow sign-in"]),
    ] {
        let list = common::json_of(
            app.api_get(
                &format!("/api/ui/projects/payments/incidents{query}"),
                Some(&tl),
            )
            .await,
        )
        .await;
        let got: Vec<&str> = list["data"]["incidents"]
            .as_array()
            .unwrap()
            .iter()
            .map(|i| i["title"].as_str().unwrap())
            .collect();
        assert_eq!(got, titles, "{query}");
    }
}
