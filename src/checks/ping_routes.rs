//! The ping endpoints a project's jobs call. Contract: docs/api-specs/04-checks.md 4.1 and 4.2.

use std::net::SocketAddr;

use axum::{
    Router,
    extract::{Path, Request, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};

use super::heartbeat::{self, Admission, PingKind};
use crate::{
    app::AppState,
    platform::{client_ip, peer_addr},
};

/// A stalled upload must not hold the failure back: past this, the cause is what arrived.
const BODY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
/// An admitted `/fail` body is read to its end, so `curl -f` sees the answer instead of a broken
/// pipe; only its first 1 KB is kept. Past this, Monita stops reading and answers anyway.
const BODY_DRAIN_MAX: usize = 16 * 1024 * 1024;

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
    let admitted = match heartbeat::admit(&state.db, &state.limiter, &p).await {
        Ok(Admission::Admitted(a)) => a,
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
        Some(body) => read_cause(body, BODY_TIMEOUT).await,
        None => None,
    };
    match heartbeat::record(&state.db, &state.config.base_url, admitted, cause).await {
        Ok(true) => plain(StatusCode::OK, "OK"),
        Ok(false) => plain(StatusCode::NOT_FOUND, "not found"),
        Err(e) => failed(e),
    }
}

/// Feeds the body to `heartbeat::Cause`, which keeps only its start. `DefaultBodyLimit` does not
/// reach a raw body, so this loop is the bound: it stops at `BODY_DRAIN_MAX` or `limit`.
async fn read_cause(mut body: axum::body::Body, limit: std::time::Duration) -> Option<String> {
    use axum::body::HttpBody;
    let mut cause = heartbeat::Cause::default();
    let read = async {
        let mut seen = 0usize;
        while seen < BODY_DRAIN_MAX {
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
    if tokio::time::timeout(limit, read).await.is_err() {
        tracing::warn!("a /fail body stalled; keeping what arrived");
    }
    cause.finish()
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
        peer_addr(req.extensions()),
        None,
    )
    .await
}

async fn ping_fail(
    State(state): State<AppState>,
    Path(token): Path<String>,
    req: Request,
) -> Response {
    let (parts, body) = req.into_parts();
    let peer = peer_addr(&parts.extensions);
    ping(
        &state,
        &token,
        PingKind::Fail,
        &parts.headers,
        peer,
        Some(body),
    )
    .await
}

#[cfg(test)]
mod tests {
    use std::{
        pin::Pin,
        task::{Context, Poll},
        time::Duration,
    };

    use axum::body::Bytes;

    /// Sends one chunk, then nothing, as a stuck upload does.
    struct Stalls(bool);

    impl http_body::Body for Stalls {
        type Data = Bytes;
        type Error = std::convert::Infallible;

        fn poll_frame(
            mut self: Pin<&mut Self>,
            _: &mut Context<'_>,
        ) -> Poll<Option<Result<http_body::Frame<Bytes>, Self::Error>>> {
            if std::mem::replace(&mut self.0, true) {
                Poll::Pending
            } else {
                Poll::Ready(Some(Ok(http_body::Frame::data(Bytes::from("Error: half")))))
            }
        }
    }

    #[tokio::test]
    async fn a_stalled_body_keeps_what_arrived_at_the_time_limit() {
        let body = axum::body::Body::new(Stalls(false));
        let cause = super::read_cause(body, Duration::from_millis(50)).await;
        assert_eq!(cause.as_deref(), Some("Error: half"));
    }
}
