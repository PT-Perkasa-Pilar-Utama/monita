//! Project reports as PDF (v1.2 Q56 to Q60): the summary of a range of days and the report of
//! one incident, read with the numbers every other screen shows.

mod common;

use std::io::Read as _;

use common::TestApp;

// 2026-10-01 00:00 UTC; the admin offset is UTC by default.
const OCT_1: i64 = 1_790_812_800;
const DAY: i64 = 86_400;

async fn seed(app: &TestApp) {
    let sql = format!(
        "INSERT INTO monitors (id, project_id, name, url, next_check_at, created_at) VALUES
           (1, 1, 'API', 'https://api.invalid/health?token=secret', 9999999999, {old}),
           (2, 1, 'Site', 'https://site.invalid', 9999999999, {old});
         INSERT INTO heartbeats (id, project_id, name, token, period_s, grace_s, created_at)
           VALUES (1, 1, 'Backup', 'tok', 3600, 0, {old});
         INSERT INTO incidents (id, project_id, origin, title, severity, cause, started_at, resolved_at, excluded) VALUES
           (1, 1, 'auto', 'API is down', 'critical', 'HTTP 503 (from) \\ edge', {a}, {a} + 3600, NULL),
           (2, 1, 'auto', 'Site is slow', 'warning', NULL, {b}, {b} + 600, 'planned'),
           (3, 1, 'manual', 'Payments degraded', 'warning', NULL, {c}, {c} + 7200, NULL);
         INSERT INTO incident_items VALUES (1, 'monitor', 1), (2, 'monitor', 2), (3, 'heartbeat', 1);
         INSERT INTO status_updates (incident_id, label, message, author_id, created_at) VALUES
           (1, 'investigating', 'Detected: HTTP 503.', NULL, {a}),
           (1, 'resolved', 'The edge cache was flushed.', (SELECT min(id) FROM users), {a} + 3600);
         INSERT INTO maintenance_windows (id, project_id, title, message, starts_at, ends_at, created_at)
           VALUES (1, 1, 'DNS move', '', {d}, {d} + 3600, {d});",
        old = OCT_1 - 30 * DAY,
        a = OCT_1 + 10 * 3600,
        b = OCT_1 + 4 * DAY,
        c = OCT_1 + 9 * DAY,
        d = OCT_1 + 19 * DAY,
    );
    app.state
        .db
        .write(async move |c| {
            c.execute_batch(&sql).await?;
            Ok(())
        })
        .await
        .unwrap();
}

/// The text of every page: each content stream inflated.
fn pages(pdf: &[u8]) -> String {
    let mut out = String::new();
    let mut at = 0;
    while let Some(i) = find(&pdf[at..], b">>\nstream\n") {
        let start = at + i + 10;
        let end = start + find(&pdf[start..], b"\nendstream").unwrap();
        let mut text = String::new();
        flate2::read::ZlibDecoder::new(&pdf[start..end])
            .read_to_string(&mut text)
            .unwrap();
        out.push_str(&text);
        at = end + 10;
    }
    out
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

/// Keeps a copy for a person to look at when `MONITA_REPORT_OUT` names a folder.
fn keep(name: &str, pdf: &[u8]) {
    if let Ok(dir) = std::env::var("MONITA_REPORT_OUT") {
        std::fs::write(format!("{dir}/{name}"), pdf).unwrap();
    }
}

#[tokio::test]
async fn the_summary_report_counts_automatic_incidents_only_as_downtime() {
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    app.project(&tl, "Payments", "payments").await;
    seed(&app).await;

    let res = app
        .get(
            "/api/ui/projects/payments/report?from=2026-10-01&to=2026-10-31",
            Some(&tl),
        )
        .await;
    assert_eq!(res.status(), 200);
    assert_eq!(res.headers()["content-type"], "application/pdf");
    assert_eq!(
        res.headers()["content-disposition"],
        "attachment; filename=\"payments-2026-10-01-to-2026-10-31.pdf\""
    );
    let pdf = res.bytes().await.unwrap();
    keep("summary.pdf", &pdf);
    assert!(pdf.starts_with(b"%PDF-1.4"));
    let text = pages(&pdf);
    // API down 3 600 s of 31 days: 100 * (1 - 3600 / 2 678 400) = 99.87 %.
    for want in [
        "(Payments report)",
        "(Outages)",
        "(This report covers 2026-10-01 to 2026-10-31, and its times are in UTC. Monita made it on",
        "(API)",
        "(99.87%)",
        "(Planned work)",
        "(Payments degraded)",
        "(DNS move)",
        "(1h 0m)",
    ] {
        assert!(text.contains(want), "missing {want}");
    }
    // Site's incident is planned work and the manual one is not downtime: both read 100 %.
    assert_eq!(text.matches("(100.00%)").count(), 2, "Site and Backup");
    assert!(!text.contains("token=secret"), "no monitor URL in a report");
    assert!(!text.contains("api.invalid"));
}

#[tokio::test]
async fn the_incident_report_lists_its_systems_cause_and_updates() {
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    app.project(&tl, "Payments", "payments").await;
    seed(&app).await;

    let res = app
        .get("/api/ui/projects/payments/incidents/1/report", Some(&tl))
        .await;
    assert_eq!(res.status(), 200);
    assert_eq!(
        res.headers()["content-disposition"],
        "attachment; filename=\"payments-inc-1.pdf\""
    );
    let pdf = res.bytes().await.unwrap();
    keep("incident.pdf", &pdf);
    let text = pages(&pdf);
    for want in [
        "(INC-1: API is down)",
        "(Monitor: API)",
        r"(HTTP 503 \(from\) \\ edge)",
        "(Monita)",
        "(rina@x.co)",
        "(The edge cache was flushed.)",
        "(Critical)",
        "(1h 0m)",
    ] {
        assert!(text.contains(want), "missing {want}");
    }
}

#[tokio::test]
async fn a_report_is_refused_for_a_bad_range_another_owner_or_a_missing_incident() {
    let app = TestApp::spawn("test").await;
    let rina = app.tl("rina@x.co").await;
    let budi = app.tl("budi@x.co").await;
    let boss = app.super_admin("boss@x.co").await;
    app.project(&rina, "Payments", "payments").await;
    seed(&app).await;

    let status = async |path: &str, cookie: Option<&str>| app.get(path, cookie).await.status();
    let base = "/api/ui/projects/payments";
    assert_eq!(
        status(
            &format!("{base}/report?from=2026-10-05&to=2026-10-01"),
            Some(&rina)
        )
        .await,
        422
    );
    assert_eq!(
        status(
            &format!("{base}/report?from=2024-01-01&to=2026-10-01"),
            Some(&rina)
        )
        .await,
        422
    );
    assert_eq!(status(&format!("{base}/report"), Some(&rina)).await, 422);
    assert_eq!(
        status(&format!("{base}/report?period=month"), Some(&budi)).await,
        404
    );
    assert_eq!(
        status(&format!("{base}/report?period=month"), None).await,
        401
    );
    assert_eq!(
        status(&format!("{base}/incidents/99/report"), Some(&rina)).await,
        404
    );
    for period in ["week", "7d", "month", "quarter"] {
        assert_eq!(
            status(&format!("{base}/report?period={period}"), Some(&boss)).await,
            200,
            "{period}"
        );
    }
}

/// The PDF's text items in drawing order, unescaped.
fn literals(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '(' {
            continue;
        }
        let mut s = String::new();
        while let Some(c) = chars.next() {
            match c {
                '\\' => s.extend(chars.next()),
                ')' => break,
                _ => s.push(c),
            }
        }
        out.push(s);
    }
    out
}

/// `whole` put back together from the wrapped lines that are pieces of it.
fn rebuilt(text: &str, whole: &str) -> String {
    literals(text)
        .into_iter()
        // Short items such as a figure "0" can sit inside `whole` by chance.
        .filter(|l| l.trim().len() > 2 && whole.contains(l.trim()))
        .collect::<Vec<_>>()
        .concat()
        .replace(' ', "")
}

#[tokio::test]
async fn long_names_titles_emails_and_tokens_are_wrapped_never_cut() {
    let app = TestApp::spawn("test").await;
    let name = "Payments platform for the Jakarta and Surabaya regional offices second rollout";
    let email = "rina.kartika.wijayakusuma@corp.co.id";
    let tl = app.tl(email).await;
    app.project(&tl, name, "payments").await;
    let title = "Checkout and refunds fail for card payments while the acquirer gateway in the Jakarta region answers with HTTP 503 errors";
    let token: String = (0..150)
        .map(|i| char::from(b'a' + (i % 26) as u8))
        .collect();
    let sql = format!(
        "INSERT INTO incidents (id, project_id, origin, title, severity, started_at, resolved_at) VALUES
           (1, 1, 'manual', '{title}', 'critical', {a}, {a} + 600);
         INSERT INTO status_updates (incident_id, label, message, author_id, created_at) VALUES
           (1, 'identified', '{token}', (SELECT id FROM users WHERE email = '{email}'), {a});",
        a = OCT_1,
    );
    app.state
        .db
        .write(async move |c| {
            c.execute_batch(&sql).await?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(name.len(), 78);
    assert_eq!(title.len(), 121);

    let pdf = app
        .get("/api/ui/projects/payments/incidents/1/report", Some(&tl))
        .await
        .bytes()
        .await
        .unwrap();
    keep("long-incident.pdf", &pdf);
    let text = pages(&pdf);
    let full = format!("INC-1: {title}");
    assert_eq!(
        rebuilt(&text, &full),
        full.replace(' ', ""),
        "the whole title"
    );
    assert_eq!(rebuilt(&text, &token), token, "the whole token");
    assert_eq!(rebuilt(&text, email), email, "the whole author");

    let pdf = app
        .get(
            "/api/ui/projects/payments/report?from=2026-10-01&to=2026-10-31",
            Some(&tl),
        )
        .await
        .bytes()
        .await
        .unwrap();
    keep("long-summary.pdf", &pdf);
    let text = pages(&pdf);
    let heading = format!("{name} report");
    assert_eq!(rebuilt(&text, &heading), heading.replace(' ', ""));
    assert_eq!(
        rebuilt(&text, title),
        title.replace(' ', ""),
        "the incident row"
    );
}
