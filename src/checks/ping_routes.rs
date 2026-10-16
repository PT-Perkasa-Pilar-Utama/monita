//! The ping endpoints a project's jobs call. Contract: docs/api-specs/04-checks.md 4.1 and 4.2.

use axum::{
    Router,
    extract::{Path, Request, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};

use super::heartbeat::{self, Admission, PingKind};
use crate::{app::AppState, platform::caller_ip};

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

/// Admits the ping; for `/fail`, reads the first 1 KB of the body as the cause, records the ping
/// and its incident, then reads the rest without keeping it, so a slow or large upload never
/// delays the incident and `curl -f` still gets its answer.
async fn ping(
    state: &AppState,
    token: &str,
    kind: PingKind,
    headers: &HeaderMap,
    extensions: &axum::http::Extensions,
    body: Option<axum::body::Body>,
) -> Response {
    let p = heartbeat::Ping {
        token,
        kind,
        source_ip: caller_ip(headers, extensions, state.config.trust_proxy),
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
    let mut body = body;
    let cause = match body.as_mut() {
        Some(b) => read_cause(b, BODY_TIMEOUT).await,
        None => None,
    };
    let stored = heartbeat::record(&state.db, &state.config.base_url, admitted, cause).await;
    if let Some(b) = body {
        drain(b, BODY_DRAIN_MAX, BODY_TIMEOUT).await;
    }
    match stored {
        Ok(true) => plain(StatusCode::OK, "OK"),
        Ok(false) => plain(StatusCode::NOT_FOUND, "not found"),
        Err(e) => failed(e),
    }
}

/// The next data chunk, or `None` at the end. A broken body ends early, with a log line.
async fn next_chunk(body: &mut axum::body::Body) -> Option<axum::body::Bytes> {
    use axum::body::HttpBody;
    loop {
        match std::future::poll_fn(|cx| std::pin::Pin::new(&mut *body).poll_frame(cx)).await? {
            Ok(frame) => {
                if let Ok(data) = frame.into_data() {
                    return Some(data);
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "a /fail body broke off; keeping what arrived");
                return None;
            }
        }
    }
}

/// Reads until `heartbeat::Cause` has its 1 KB, the body ends, or `limit` passes.
async fn read_cause(body: &mut axum::body::Body, limit: std::time::Duration) -> Option<String> {
    let mut cause = heartbeat::Cause::default();
    let read = async {
        while !cause.is_full() {
            let Some(data) = next_chunk(body).await else {
                break;
            };
            cause.push(&data);
        }
    };
    if tokio::time::timeout(limit, read).await.is_err() {
        tracing::warn!("a /fail body stalled; keeping what arrived");
    }
    cause.finish()
}

/// Reads and drops the rest of the body, up to `max` bytes or `limit`. `DefaultBodyLimit` does
/// not reach a raw body, so this is the bound.
async fn drain(mut body: axum::body::Body, max: usize, limit: std::time::Duration) {
    let read = async {
        let mut seen = 0usize;
        while seen < max {
            let Some(data) = next_chunk(&mut body).await else {
                break;
            };
            seen += data.len();
        }
    };
    let _ = tokio::time::timeout(limit, read).await;
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
        req.extensions(),
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
    let (headers, extensions) = (&parts.headers, &parts.extensions);
    ping(
        &state,
        &token,
        PingKind::Fail,
        headers,
        extensions,
        Some(body),
    )
    .await
}

#[cfg(test)]
mod tests {
    use std::{
        pin::Pin,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        task::{Context, Poll},
        time::Duration,
    };

    use axum::body::{Body, Bytes};

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

    /// 4 bytes per frame for 40 KB, counting the frames read.
    struct Long(Arc<AtomicUsize>);

    impl http_body::Body for Long {
        type Data = Bytes;
        type Error = std::convert::Infallible;

        fn poll_frame(
            self: Pin<&mut Self>,
            _: &mut Context<'_>,
        ) -> Poll<Option<Result<http_body::Frame<Bytes>, Self::Error>>> {
            let more = self.0.fetch_add(1, Ordering::SeqCst) < 10_000;
            Poll::Ready(more.then(|| Ok(http_body::Frame::data(Bytes::from("xxxx")))))
        }
    }

    #[tokio::test]
    async fn a_stalled_body_keeps_what_arrived_at_the_time_limit() {
        let mut body = Body::new(Stalls(false));
        let cause = super::read_cause(&mut body, Duration::from_millis(50)).await;
        assert_eq!(cause.as_deref(), Some("Error: half"));
    }

    #[tokio::test]
    async fn the_cause_stops_reading_at_1_kb_and_the_drain_at_its_byte_cap() {
        let frames = Arc::new(AtomicUsize::new(0));
        let mut body = Body::new(Long(frames.clone()));
        let cause = super::read_cause(&mut body, Duration::from_secs(60)).await;
        assert_eq!(cause.map(|c| c.len()), Some(1024));
        let read = frames.swap(0, Ordering::SeqCst);
        assert!(read < 300, "read {read} frames for a 1 KB cause");
        super::drain(body, 10, Duration::from_secs(60)).await;
        assert_eq!(frames.load(Ordering::SeqCst), 3, "4 + 4 + 4 bytes reach 10");
    }
}
