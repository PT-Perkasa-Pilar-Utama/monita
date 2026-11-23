//! The compact Projects view (v1.2 Q53): 100 projects a page, each with the state its card shows.

mod common;

use common::{TestApp, json_of};
use serde_json::Value;

async fn grid(app: &TestApp, cookie: &str, query: &str) -> Value {
    let res = app
        .api_get(&format!("/api/ui/projects-grid{query}"), Some(cookie))
        .await;
    assert_eq!(res.status(), 200);
    json_of(res).await["data"].clone()
}

fn tiles(d: &Value) -> Vec<String> {
    d["projects"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            format!(
                "{} {}",
                p["slug"].as_str().unwrap(),
                p["kind"].as_str().unwrap()
            )
        })
        .collect()
}

#[tokio::test]
async fn the_compact_view_shows_each_projects_card_state_100_a_page() {
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    for (name, slug) in [("Down", "down"), ("Issue", "issue"), ("Fine", "fine")] {
        app.project(&tl, name, slug).await;
    }
    app.state
        .db
        .write(async |c| {
            c.execute_batch(
                "INSERT INTO monitors (id, project_id, name, url, next_check_at) VALUES
                   (1, 1, 'A', 'https://x.invalid', 9999999999), (2, 2, 'B', 'https://x.invalid', 9999999999),
                   (3, 3, 'C', 'https://x.invalid', 9999999999);
                 INSERT INTO incidents (id, project_id, origin, title, severity, started_at) VALUES
                   (1, 1, 'auto', 'A is down', 'critical', 1), (2, 2, 'manual', 'B is slow', 'warning', 1);
                 INSERT INTO incident_items VALUES (1, 'monitor', 1), (2, 'monitor', 2);",
            )
            .await?;
            Ok(())
        })
        .await
        .unwrap();

    let d = grid(&app, &tl, "?sort=name").await;
    assert_eq!(d["page_size"], 100);
    assert_eq!(d["total"], 3);
    assert_eq!(tiles(&d), ["down down", "fine unknown", "issue issue"]);
    let list = json_of(app.api_get("/api/ui/projects?sort=name", Some(&tl)).await).await;
    let cards: Vec<String> = list["data"]["projects"]
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
    assert_eq!(tiles(&d), cards, "a tile reads the state its card shows");

    let first = &d["projects"][0];
    assert!(
        first["icon_url"].is_null(),
        "no icon: the page shows the first letter"
    );
    assert_eq!(first["name"], "Down");
    assert_eq!(first["pinned"], false);
    assert_eq!(tiles(&grid(&app, &tl, "?state=down").await), ["down down"]);
    assert_eq!(tiles(&grid(&app, &tl, "?q=iss").await), ["issue issue"]);
}

#[tokio::test]
async fn the_compact_view_needs_a_session_and_shows_only_a_tech_leads_own_projects() {
    let app = TestApp::spawn("test").await;
    let rina = app.tl("rina@x.co").await;
    let budi = app.tl("budi@x.co").await;
    app.project(&rina, "Shop", "shop").await;
    app.project(&budi, "Search", "search").await;
    assert_eq!(
        app.api_get("/api/ui/projects-grid", None).await.status(),
        401
    );
    let d = grid(&app, &rina, "").await;
    assert_eq!(d["total"], 1);
    assert_eq!(d["projects"][0]["slug"], "shop");
}
