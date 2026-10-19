//! A project's releases: the team writes them, the public Changelog tab shows them newest first.

mod common;

use common::{TestApp, json_of};
use serde_json::{Value, json};

const TEAM: &str = "/api/ui/projects/payments/changelog";
const PUBLIC: &str = "/api/ui/public/payments/changelog";

fn release(version: &str, day: &str) -> Value {
    json!({ "version": version, "released_on": day, "title": format!("Release {version}"), "added": "One thing\nAnother" })
}

async fn tab(app: &TestApp, cookie: &str, on: bool) {
    let res = app
        .api_send(
            reqwest::Method::PUT,
            "/api/ui/projects/payments/public-pages",
            json!({ "incidents": true, "known_issues": true, "report": true, "changelog": on }),
            Some(cookie),
        )
        .await;
    assert_eq!(res.status(), 204);
}

#[tokio::test]
async fn the_team_writes_releases_and_a_bad_form_names_every_field() {
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    app.project(&tl, "Payments", "payments").await;
    app.project(&tl, "Search", "search").await;

    let bad = app
        .api_post(
            TEAM,
            json!({ "version": "a b", "released_on": "soon" }),
            Some(&tl),
        )
        .await;
    assert_eq!(bad.status(), 422);
    let fields = json_of(bad).await["error"]["fields"].clone();
    for f in ["version", "released_on", "title", "added"] {
        assert!(fields[f].is_string(), "{f} missing from {fields}");
    }

    assert_eq!(
        app.api_post(TEAM, release("1.0.0", "2026-09-01"), Some(&tl))
            .await
            .status(),
        201
    );
    let dup = app
        .api_post(TEAM, release("1.0.0", "2026-09-02"), Some(&tl))
        .await;
    assert_eq!(dup.status(), 422);
    assert!(json_of(dup).await["error"]["fields"]["version"].is_string());
    let other = app
        .api_post(
            "/api/ui/projects/search/changelog",
            release("1.0.0", "2026-09-01"),
            Some(&tl),
        )
        .await;
    assert_eq!(other.status(), 201, "versions are unique per project only");

    let stranger = app.tl("budi@x.co").await;
    assert_eq!(app.api_get(TEAM, Some(&stranger)).await.status(), 404);
    let list = json_of(app.api_get(TEAM, Some(&tl)).await).await;
    assert_eq!(
        list["data"]["releases"][0]["sections"][0]["items"][1],
        "Another"
    );
}

#[tokio::test]
async fn the_public_tab_shows_releases_newest_first_only_when_on() {
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    app.project(&tl, "Payments", "payments").await;
    for (v, d) in [
        ("1.0.0", "2026-09-01"),
        ("1.2.0", "2026-10-01"),
        ("1.1.0", "2026-09-15"),
    ] {
        assert_eq!(
            app.api_post(TEAM, release(v, d), Some(&tl)).await.status(),
            201
        );
    }
    assert_eq!(
        app.get(PUBLIC, None).await.status(),
        404,
        "the tab starts off"
    );

    tab(&app, &tl, true).await;
    let index = json_of(app.get(PUBLIC, None).await).await;
    let versions: Vec<_> = index["data"]["releases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["version"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(versions, ["1.2.0", "1.1.0", "1.0.0"]);
    let found = json_of(app.get(&format!("{PUBLIC}?q=1.1"), None).await).await;
    assert_eq!(
        found["data"]["releases"].as_array().unwrap().len(),
        1,
        "the search runs on every release, not a loaded page"
    );
    assert_eq!(found["data"]["releases"][0]["version"], "1.1.0");
    let newest = json_of(app.get(&format!("{PUBLIC}/latest"), None).await).await;
    assert_eq!(newest["data"]["release"]["version"], "1.2.0");
    let middle = json_of(app.get(&format!("{PUBLIC}/1.1.0"), None).await).await;
    assert_eq!(middle["data"]["newer"]["version"], "1.2.0");
    assert_eq!(middle["data"]["older"]["version"], "1.0.0");

    tab(&app, &tl, false).await;
    assert_eq!(
        app.get(&format!("{PUBLIC}/1.1.0"), None).await.status(),
        404
    );
}
