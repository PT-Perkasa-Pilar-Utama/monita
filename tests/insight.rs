//! The monitor and heartbeat pages' numbers: availability periods, the streak, the month chart.

mod common;

use common::TestApp;
use monita::{
    checks::{chart::Range, series},
    incidents::{ItemRef, SourceKind, availability},
    platform::{now, time},
};

async fn sql(app: &TestApp, sql: &str) {
    let sql = sql.to_string();
    app.state
        .db
        .write(async |c| {
            c.execute_batch(&sql).await?;
            Ok(())
        })
        .await
        .unwrap();
}

/// A TL, project 1 and monitor 1 created `age` seconds ago.
async fn monitor(app: &TestApp, age: i64) -> i64 {
    let tl = app.tl("rina@x.co").await;
    app.post_form(
        "/projects",
        &[("name", "Payments"), ("slug", "payments")],
        Some(&tl),
    )
    .await;
    let t = now();
    sql(app, &format!("INSERT INTO monitors (id, project_id, name, url, next_check_at, created_at) VALUES (1, 1, 'API', 'https://x.co', 9999999999, {});", t - age)).await;
    t
}

#[tokio::test]
async fn availability_counts_automatic_incidents_inside_each_period() {
    let app = TestApp::spawn("test").await;
    let t = monitor(&app, 40 * 86_400).await;
    // One hour down two days ago; a manual incident is never downtime.
    sql(&app, &format!(
        "INSERT INTO incidents (id, project_id, origin, title, severity, started_at, resolved_at) VALUES
           (1, 1, 'auto', 'down', 'critical', {a}, {b}), (2, 1, 'manual', 'slow', 'warning', {a}, NULL);
         INSERT INTO incident_items VALUES (1, 'monitor', 1), (2, 'monitor', 1);",
        a = t - 2 * 86_400,
        b = t - 2 * 86_400 + 3_600
    ))
    .await;
    let item = ItemRef {
        kind: SourceKind::Monitor,
        id: 1,
    };
    let r = availability::report(&app.state.db, item, t - 40 * 86_400, t)
        .await
        .unwrap();
    let week = &r.periods[1];
    assert_eq!(week.label, "Last 7 days");
    assert_eq!(
        (week.incidents, week.downtime_s, week.longest_s),
        (1, 3_600, Some(3_600))
    );
    let expected = 100.0 * (1.0 - 3_600.0 / (7.0 * 86_400.0));
    assert!((week.availability.unwrap() - expected).abs() < 1e-9);
    assert_eq!(r.periods[0].incidents, 0, "today has none");
    assert_eq!(r.periods[4].incidents, 1, "all time");
    assert!(!r.streak.down);
    assert_eq!(
        r.streak.since,
        t - 2 * 86_400 + 3_600,
        "up since the incident ended"
    );
}

#[tokio::test]
async fn an_open_incident_makes_the_streak_down_since_it_started() {
    let app = TestApp::spawn("test").await;
    let t = monitor(&app, 86_400).await;
    sql(&app, &format!(
        "INSERT INTO incidents (id, project_id, origin, title, severity, started_at) VALUES (1, 1, 'auto', 'down', 'critical', {});
         INSERT INTO incident_items VALUES (1, 'monitor', 1);",
        t - 600
    ))
    .await;
    let item = ItemRef {
        kind: SourceKind::Monitor,
        id: 1,
    };
    let r = availability::report(&app.state.db, item, t - 86_400, t)
        .await
        .unwrap();
    assert!(r.streak.down);
    assert_eq!(r.streak.since, t - 600);
    assert_eq!(r.periods[1].downtime_s, 600);
}

#[tokio::test]
async fn the_month_chart_reads_the_rollup_and_todays_checks() {
    let app = TestApp::spawn("test").await;
    let t = monitor(&app, 40 * 86_400).await;
    let three_days_ago = time::day(t - 3 * 86_400);
    sql(&app, &format!(
        "INSERT INTO daily_summaries (source_kind, source_id, day, checks_total, checks_ok, downtime_s, avg_latency_ms) VALUES ('monitor', 1, '{three_days_ago}', 1440, 1439, 0, 120);
         INSERT INTO check_results (monitor_id, checked_at, ok, latency_ms) VALUES (1, {t}, 1, 80), (1, {t}, 0, 80);"
    ))
    .await;
    let m = the_monitor(&app).await;
    let s = series::monitor(&app.state.db, &m, Range::Month, t)
        .await
        .unwrap();
    assert_eq!((s.total, s.ok), (1442, 1440));
    assert_eq!(
        s.chart.fails.len(),
        2,
        "the day with a failed check and today"
    );
    assert!(!s.chart.empty);
}

async fn the_monitor(app: &TestApp) -> monita::checks::Monitor {
    monita::checks::find_monitor(&app.state.db, 1, 1)
        .await
        .unwrap()
        .unwrap()
}
