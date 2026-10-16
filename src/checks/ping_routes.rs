//! The ping endpoints a project's jobs call. Contract: docs/api-specs/04-checks.md 4.1 and 4.2.

use std::net::SocketAddr;

use axum::{
    Router,
    extract::{ConnectInfo, Path, Request, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};

use super::heartbeat::{self, Admission, PingKind};
use crate::{app::AppState, platform::client_ip};

/// A stalled upload must not hold the failure back: past this, the cause is what arrived.
const BODY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/ping/{token}", get(ping_ok).head(ping_ok).post(ping_ok))
        .route(
            "/ping/{token}/fail",
            get(ping_fail).head(ping_fail).post(ping_fail),
        )
}

fn plain(status: StatusCode, body: &'static str) -> Response {
    (
        status,
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        body,
    )
        .into_response()
}

/// Admits the ping, reads a `/fail` body only once it is admitted, then records it.
async fn ping(
    state: &AppState,
    token: &str,
    kind: PingKind,
    headers: &HeaderMap,
    peer: Option<SocketAddr>,
    body: Option<axum::body::Body>,
) -> Response {
    let p = heartbeat::Ping {
        token,
        kind,
        source_ip: client_ip(headers, peer, state.config.trust_proxy),
    };
    let failed = |e: anyhow::Error| {
        tracing::error!(error = %e, "ping failed");
        plain(StatusCode::INTERNAL_SERVER_ERROR, "error")
    };
    let hb = match heartbeat::admit(&state.db, &state.limiter, &p).await {
        Ok(Admission::Admitted(hb)) => hb,
        Ok(Admission::UnknownToken) => return plain(StatusCode::NOT_FOUND, "not found"),
        Ok(Admission::Limited(retry)) => {
            return (
                StatusCode::TOO_MANY_REQUESTS,
                [(header::RETRY_AFTER, retry.to_string())],
                "slow down",
            )
                .into_response();
        }
        Err(e) => return failed(e),
    };
    let cause = match body {
        Some(body) => read_cause(body).await,
        None => None,
    };
    match heartbeat::record(&state.db, &state.config.base_url, &hb, &p, cause).await {
        Ok(true) => plain(StatusCode::OK, "OK"),
        Ok(false) => plain(StatusCode::NOT_FOUND, "not found"),
        Err(e) => failed(e),
    }
}

/// Feeds the body to `heartbeat::Cause`, which keeps only its start. `DefaultBodyLimit` does not
/// reach a raw body, so this loop is the bound: it stops at `MAX_BODY` or `BODY_TIMEOUT`.
async fn read_cause(mut body: axum::body::Body) -> Option<String> {
    use axum::body::HttpBody;
    let mut cause = heartbeat::Cause::default();
    let read = async {
        let mut seen = 0usize;
        while seen < crate::app::MAX_BODY {
            match std::future::poll_fn(|cx| std::pin::Pin::new(&mut body).poll_frame(cx)).await {
                None => break,
                Some(Err(e)) => {
                    tracing::warn!(error = %e, "a /fail body broke off; keeping what arrived");
                    break;
                }
                Some(Ok(frame)) => {
                    if let Ok(data) = frame.into_data() {
                        seen += data.len();
                        cause.push(&data);
                    }
                }
            }
        }
    };
    if tokio::time::timeout(BODY_TIMEOUT, read).await.is_err() {
        tracing::warn!("a /fail body stalled; keeping what arrived");
    }
    cause.finish()
}

fn peer(req: &Request) -> Option<SocketAddr> {
    req.extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|c| c.0)
}

async fn ping_ok(
    State(state): State<AppState>,
    Path(token): Path<String>,
    req: Request,
) -> Response {
    ping(
        &state,
        &token,
        PingKind::Ok,
        req.headers(),
        peer(&req),
        None,
    )
    .await
}

async fn ping_fail(
    State(state): State<AppState>,
    Path(token): Path<String>,
    req: Request,
) -> Response {
    let (headers, peer) = (req.headers().clone(), peer(&req));
    let body = Some(req.into_body());
    ping(&state, &token, PingKind::Fail, &headers, peer, body).await
}
