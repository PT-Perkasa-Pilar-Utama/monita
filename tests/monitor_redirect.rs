//! A check that sends its own headers, or a header-kind project secret, follows a redirect only
//! while it stays on the monitored server (security audit 2026-10-27, SEC-23).

use std::sync::{Arc, Mutex};

use axum::{
    Router,
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
    routing::get,
};
use monita::checks::{Auth, Clients, Target, probe};

/// The x-api-key, x-env and authorization headers of each request a server got.
type Seen = Arc<Mutex<Vec<[String; 3]>>>;

fn read(h: &HeaderMap) -> [String; 3] {
    ["x-api-key", "x-env", "authorization"].map(|n| {
        h.get(n)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string()
    })
}

/// A server whose `/landing` records its headers and answers 200, `/same` redirects to its own
/// `/landing`, and `/away` redirects to `away`.
async fn server(away: Option<String>) -> (String, Seen) {
    let seen: Seen = Arc::default();
    let app = Router::new()
        .route(
            "/landing",
            get(
                |State((s, _)): State<(Seen, Option<String>)>, h: HeaderMap| async move {
                    s.lock().unwrap().push(read(&h));
                    "ok"
                },
            ),
        )
        .route(
            "/same",
            get(|| async { (StatusCode::FOUND, [(header::LOCATION, "/landing")]).into_response() }),
        )
        .route(
            "/away",
            get(
                |State((_, away)): State<(Seen, Option<String>)>| async move {
                    let to = away.unwrap_or_default();
                    (StatusCode::FOUND, [(header::LOCATION, to)]).into_response()
                },
            ),
        )
        .with_state((seen.clone(), away));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (format!("http://{addr}"), seen)
}

fn target<'a>(url: &'a str, headers: &[(&str, &str)], auth: Option<Auth<'a>>) -> Target<'a> {
    Target {
        url,
        method: "GET",
        headers: headers
            .iter()
            .map(|(n, v)| (n.to_string(), v.to_string()))
            .collect(),
        body: None,
        timeout_ms: 5_000,
        expected_status: "2xx",
        keyword: None,
        skip_tls_verify: false,
        auth,
    }
}

#[tokio::test]
async fn a_monitors_own_headers_never_follow_a_redirect_to_another_server() {
    let (other, other_seen) = server(None).await;
    let (base, base_seen) = server(Some(format!("{other}/landing"))).await;
    let clients = Clients::new().unwrap();
    let secret = Auth::Header {
        name: "X-Api-Key",
        value: "dummy-secret",
    };

    let away = format!("{base}/away");
    let r = probe(&target(&away, &[("X-Env", "prod")], Some(secret)), &clients).await;
    assert!(!r.ok);
    assert_eq!(r.status_code, Some(302));
    assert!(
        r.error.as_deref().unwrap_or("").contains("another server"),
        "{:?}",
        r.error
    );
    assert!(
        other_seen.lock().unwrap().is_empty(),
        "the other server got nothing"
    );

    let r = probe(&target(&away, &[("X-Env", "prod")], None), &clients).await;
    assert!(!r.ok, "user headers are guarded too");
    assert!(other_seen.lock().unwrap().is_empty());

    let same = format!("{base}/same");
    let secret = Auth::Header {
        name: "X-Api-Key",
        value: "dummy-secret",
    };
    let r = probe(&target(&same, &[("X-Env", "prod")], Some(secret)), &clients).await;
    assert!(r.ok, "{:?}", r.error);
    assert_eq!(
        base_seen.lock().unwrap().last().unwrap()[..2],
        ["dummy-secret".to_string(), "prod".to_string()],
        "a redirect on the same server keeps them"
    );
}

#[tokio::test]
async fn a_check_without_its_own_headers_still_follows_a_redirect_anywhere() {
    let (other, other_seen) = server(None).await;
    let (base, _) = server(Some(format!("{other}/landing"))).await;
    let clients = Clients::new().unwrap();
    let away = format!("{base}/away");

    let r = probe(&target(&away, &[], None), &clients).await;
    assert!(r.ok, "{:?}", r.error);

    let r = probe(
        &target(&away, &[], Some(Auth::Bearer("dummy-token"))),
        &clients,
    )
    .await;
    assert!(r.ok, "{:?}", r.error);
    let seen = other_seen.lock().unwrap();
    assert_eq!(seen.len(), 2);
    assert!(
        seen.iter().all(|h| h[2].is_empty()),
        "reqwest drops Authorization on the way to another server: {seen:?}"
    );
}
