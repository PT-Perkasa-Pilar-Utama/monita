//! The pages above the projects: open incidents and Triage tickets across every project a user
//! may open, and the counts beside the top menu.

mod common;

use common::{TestApp, json_of};
use serde_json::Value;

async fn read(app: &TestApp, path: &str, cookie: &str) -> Value {
    json_of(app.api_get(path, Some(cookie)).await).await["data"].clone()
}

fn titles(rows: &Value) -> Vec<String> {
    rows.as_array()
        .unwrap()
        .iter()
        .map(|r| {
            format!(
                "{} {}",
                r["project"]["slug"].as_str().unwrap(),
                r["title"].as_str().unwrap()
            )
        })
        .collect()
}

#[tokio::test]
async fn a_tech_lead_sees_their_own_projects_and_a_super_admin_sees_all() {
    let app = TestApp::spawn("test").await;
    let admin = app.super_admin("boss@x.co").await;
    let rina = app.tl("rina@x.co").await;
    let budi = app.tl("budi@x.co").await;
    app.project(&rina, "Payments", "payments").await;
    app.project(&budi, "Search", "search").await;
    app.state
        .db
        .write(async |c| {
            c.execute_batch(
                "INSERT INTO incidents (id, project_id, origin, title, severity, started_at) VALUES
                   (1, 1, 'manual', 'Slow checkout', 'warning', 100),
                   (2, 1, 'manual', 'Card errors', 'critical', 50),
                   (3, 2, 'manual', 'Search is down', 'critical', 200);
                 INSERT INTO incidents (id, project_id, origin, title, severity, started_at, resolved_at) VALUES
                   (4, 1, 'manual', 'Old outage', 'critical', 10, 20);
                 INSERT INTO tickets (project_id, number, created_by, stage, title, description, created_at, updated_at) VALUES
                   (1, 1, 2, 'triage', 'Refund stuck', 'd', 1, 1),
                   (1, 2, 2, 'backlog', 'Sorted already', 'd', 2, 2),
                   (2, 1, 3, 'triage', 'Typo search', 'd', 3, 3);",
            )
            .await?;
            Ok(())
        })
        .await
        .unwrap();

    let mine = read(&app, "/api/ui/incidents", &rina).await;
    assert_eq!(
        titles(&mine["incidents"]),
        ["payments Card errors", "payments Slow checkout"],
        "own projects only, open only, critical first"
    );
    let all = read(&app, "/api/ui/incidents", &admin).await;
    assert_eq!(
        titles(&all["incidents"]),
        [
            "search Search is down",
            "payments Card errors",
            "payments Slow checkout"
        ]
    );
    let triage = read(&app, "/api/ui/tickets", &rina).await;
    assert_eq!(titles(&triage["tickets"]), ["payments Refund stuck"]);
    let _ = budi;
}
