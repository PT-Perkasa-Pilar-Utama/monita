//! The ping endpoints a project's jobs call. Contract: docs/api-specs/04-checks.md 4.1 and 4.2.

use axum::{
    Router,
    body::Body,
    extract::{Path, Request, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use tokio::time::Instant;

use super::heartbeat::{self, Admission, PingKind};
use crate::{app::AppState, platform::caller_ip};

/// The time both reads of a ping body share: past it, the cause is what arrived and Monita answers.
const BODY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
/// The bytes of a ping body Monita reads at most. It reads to the end, so `curl -f` sees the
/// answer instead of a reset; only the first 1 KB of a `/fail` body is kept.
const BODY_MAX: usize = 16 * 1024 * 1024;

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

/// Admits the ping; for `/fail`, reads the first 1 KB of the body as the cause. Records the
/// ping and its incident, then reads the rest of the body without keeping it, so a slow or large
/// upload never delays the incident and `curl -f` still gets its answer. A refused ping is
/// answered without reading its body.
async fn ping(state: &AppState, token: &str, kind: PingKind, req: Request) -> Response {
    let (parts, mut body) = req.into_parts();
    let p = heartbeat::Ping {
        token,
        kind,
        source_ip: caller_ip(&parts.headers, &parts.extensions, state.config.trust_proxy),
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
    let deadline = Instant::now() + BODY_TIMEOUT;
    let (cause, seen) = match kind {
        PingKind::Fail => read_cause(&mut body, deadline, BODY_MAX).await,
        PingKind::Ok => (None, 0),
    };
    let stored = heartbeat::record(&state.db, &state.config.base_url, admitted, cause).await;
    drain(body, deadline, BODY_MAX - seen).await;
    match stored {
        Ok(true) => plain(StatusCode::OK, "OK"),
        Ok(false) => plain(StatusCode::NOT_FOUND, "not found"),
        Err(e) => failed(e),
    }
}

/// The next data chunk, or `None` at the end. A broken body ends early, with a log line.
async fn next_chunk(body: &mut Body) -> Option<axum::body::Bytes> {
    use axum::body::HttpBody;
    loop {
        match std::future::poll_fn(|cx| std::pin::Pin::new(&mut *body).poll_frame(cx)).await? {
            Ok(frame) => {
                if let Ok(data) = frame.into_data() {
                    return Some(data);
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "a ping body broke off");
                return None;
            }
        }
    }
}

/// Reads until the cause has its 1 KB, the body ends, `max` bytes pass, or the deadline.
/// Returns the cause and the bytes read.
async fn read_cause(body: &mut Body, deadline: Instant, max: usize) -> (Option<String>, usize) {
    let mut cause = super::cause::Cause::default();
    let mut seen = 0;
    let read = async {
        while !cause.is_full() && seen < max {
            let Some(data) = next_chunk(body).await else {
                break;
            };
            seen += data.len();
            cause.push(&data);
        }
    };
    if tokio::time::timeout_at(deadline, read).await.is_err() {
        tracing::warn!("a /fail body stalled; keeping what arrived");
    }
    (cause.finish(), seen)
}

/// Reads and drops the rest of the body, up to `max` bytes or the deadline. `DefaultBodyLimit`
/// does not reach a raw body, so this is the bound.
async fn drain(mut body: Body, deadline: Instant, max: usize) {
    let read = async {
        let mut seen = 0;
        while seen < max {
            let Some(data) = next_chunk(&mut body).await else {
                return;
            };
            seen += data.len();
        }
        tracing::warn!(
            max,
            "a ping body passed its byte cap; answering without the rest"
        );
    };
    if tokio::time::timeout_at(deadline, read).await.is_err() {
        tracing::warn!("a ping body was still arriving at the time limit; answering anyway");
    }
}

async fn ping_ok(
    State(state): State<AppState>,
    Path(token): Path<String>,
    req: Request,
) -> Response {
    ping(&state, &token, PingKind::Ok, req).await
}

async fn ping_fail(
    State(state): State<AppState>,
    Path(token): Path<String>,
    req: Request,
) -> Response {
    ping(&state, &token, PingKind::Fail, req).await
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
    use tokio::time::Instant;

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

    /// 10 000 frames of the same 4 bytes, counting the frames read.
    struct Long(Arc<AtomicUsize>, &'static str);

    impl http_body::Body for Long {
        type Data = Bytes;
        type Error = std::convert::Infallible;

        fn poll_frame(
            self: Pin<&mut Self>,
            _: &mut Context<'_>,
        ) -> Poll<Option<Result<http_body::Frame<Bytes>, Self::Error>>> {
            let more = self.0.fetch_add(1, Ordering::SeqCst) < 10_000;
            Poll::Ready(more.then(|| Ok(http_body::Frame::data(Bytes::from(self.1)))))
        }
    }

    fn soon(ms: u64) -> Instant {
        Instant::now() + Duration::from_millis(ms)
    }

    #[tokio::test]
    async fn a_stalled_body_keeps_what_arrived_at_the_deadline() {
        let mut body = Body::new(Stalls(false));
        let (cause, seen) = super::read_cause(&mut body, soon(50), usize::MAX).await;
        assert_eq!((cause.as_deref(), seen), (Some("Error: half"), 11));
    }

    #[tokio::test]
    async fn the_cause_stops_reading_at_1_kb_and_the_drain_at_its_byte_cap() {
        let frames = Arc::new(AtomicUsize::new(0));
        let mut body = Body::new(Long(frames.clone(), "xxxx"));
        let (cause, seen) = super::read_cause(&mut body, soon(60_000), usize::MAX).await;
        assert_eq!((cause.map(|c| c.len()), seen), (Some(1024), 1024));
        frames.store(0, Ordering::SeqCst);
        super::drain(body, soon(60_000), 10).await;
        assert_eq!(frames.load(Ordering::SeqCst), 3, "4 + 4 + 4 bytes reach 10");
    }

    #[tokio::test]
    async fn a_whitespace_body_stops_the_cause_read_at_its_byte_cap() {
        let frames = Arc::new(AtomicUsize::new(0));
        let mut body = Body::new(Long(frames.clone(), "    "));
        let (cause, seen) = super::read_cause(&mut body, soon(60_000), 10).await;
        assert_eq!((cause, seen), (None, 12));
    }
}
