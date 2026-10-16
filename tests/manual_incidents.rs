//! US-20: a manual incident's form errors come in the form's order.

mod common;

use common::TestApp;
use monita::checks::{self, MonitorInput};

async fn setup(app: &TestApp) -> String {
    let tl = app.tl("rina@x.co").await;
    let res = app
        .post_form(
            "/projects",
            &[("name", "Payments"), ("slug", "payments")],
            Some(&tl),
        )
        .await;
    assert_eq!(res.status(), 303, "the project exists");
    let m = MonitorInput {
        name: "API".into(),
        url: "https://x.invalid/".into(),
        ..MonitorInput::defaults()
    };
    checks::create_monitor(&app.state.db, 1, &m).await.unwrap();
    tl
}

async fn error_for(app: &TestApp, tl: &str, form: &[(&str, &str)]) -> String {
    let res = app
        .post_form("/projects/payments/incidents", form, Some(tl))
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
