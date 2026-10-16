//! The ping endpoints a project's jobs call. Contract: docs/api-specs/04-checks.md 4.1 and 4.2.

use axum::{
    Router,
    body::{Body, HttpBody},
    extract::{Path, Request, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use tokio::time::Instant;

use super::heartbeat::{self, Admission, PingKind};
use crate::{app::AppState, platform::caller_ip};

/// The time both reads of a ping body share, counted from admission: past it, Monita answers.
const BODY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
/// The bytes of a ping body Monita reads at most. It reads up to this cap, so `curl -f` sees the
/// answer instead of a reset; only the first 1 KB of a `/fail` body is kept.
const BODY_MAX: usize = 16 * 1024 * 1024;

/// One deadline and one byte cap, shared by both reads of a ping body.
struct Budget {
    deadline: Instant,
    left: usize,
}

impl Budget {
    fn new() -> Self {
        Self {
            deadline: Instant::now() + BODY_TIMEOUT,
            left: BODY_MAX,
        }
    }

    /// The last chunk can carry a read past the cap, so this saturates.
    fn take(&mut self, n: usize) {
        self.left = self.left.saturating_sub(n);
    }
}

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

/// Admits the ping, then hands its body to `handle_body`. A ping refused at `admit` is answered
/// without reading its body.
async fn ping(state: &AppState, token: &str, kind: PingKind, req: Request) -> Response {
    let (parts, body) = req.into_parts();
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
    let record = async |cause| {
        // Reset-state and shutdown wait for a store in progress, never for an upload.
        let _pass = state.gate.read().await;
        heartbeat::record(&state.db, &state.config.base_url, admitted, cause).await
    };
    match handle_body(body, kind, Budget::new(), record).await {
        Ok(true) => plain(StatusCode::OK, "OK"),
        Ok(false) => plain(StatusCode::NOT_FOUND, "not found"),
        Err(e) => failed(e),
    }
}

/// For `/fail`, reads the first 1 KB of the body as the cause. Calls `record`, then reads the
/// rest of the body without keeping it. The incident waits for 1 KB of text after leading
/// whitespace, the end of the body, the byte cap, or the deadline; never for the rest.
async fn handle_body<T>(
    mut body: Body,
    kind: PingKind,
    mut budget: Budget,
    record: impl AsyncFnOnce(Option<String>) -> T,
) -> T {
    let cause = match kind {
        PingKind::Fail => read_cause(&mut body, &mut budget).await,
        PingKind::Ok => None,
    };
    let stored = record(cause).await;
    drain(body, &mut budget).await;
    stored
}

/// The next data chunk, or `None` at the end. A broken body ends early, with a log line.
async fn next_chunk(body: &mut Body) -> Option<axum::body::Bytes> {
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

/// Reads until the cause has its 1 KB, the body ends, or the budget runs out.
async fn read_cause(body: &mut Body, budget: &mut Budget) -> Option<String> {
    let mut cause = super::cause::Cause::default();
    let deadline = budget.deadline;
    let read = async {
        while !cause.is_full() && budget.left > 0 {
            let Some(data) = next_chunk(body).await else {
                break;
            };
            budget.take(data.len());
            cause.push(&data);
        }
    };
    if tokio::time::timeout_at(deadline, read).await.is_err() {
        tracing::warn!("a /fail body stalled; keeping what arrived");
    }
    cause.finish()
}

/// Reads and drops the rest of the body until the budget runs out. `DefaultBodyLimit` does not
/// reach a raw body, so this is the bound.
async fn drain(mut body: Body, budget: &mut Budget) {
    let deadline = budget.deadline;
    let read = async {
        while budget.left > 0 {
            let Some(data) = next_chunk(&mut body).await else {
                return;
            };
            budget.take(data.len());
        }
        if !body.is_end_stream() {
            tracing::warn!("a ping body passed its byte cap; answering without the rest");
        }
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
    detached(state, token, PingKind::Ok, req).await
}

async fn ping_fail(
    State(state): State<AppState>,
    Path(token): Path<String>,
    req: Request,
) -> Response {
    detached(state, token, PingKind::Fail, req).await
}

/// Runs the ping in its own task. hyper drops a handler when its client hangs up, and a ping
/// already admitted must still be recorded. Shutdown's `drain` waits up to 10 s for a store in
/// progress; a ping that reaches `record` after that is lost.
// ponytail: hyper drops a request whose client closes before dispatch; a hyper-util serve loop
// with half_close(true) may hear it.
// ponytail: a late ping is lost at shutdown; a task tracker on `AppState` that shutdown awaits
// would keep it.
async fn detached(state: AppState, token: String, kind: PingKind, req: Request) -> Response {
    match tokio::spawn(async move { ping(&state, &token, kind, req).await }).await {
        Ok(res) => res,
        Err(e) => {
            tracing::error!(error = %e, "ping task failed");
            plain(StatusCode::INTERNAL_SERVER_ERROR, "error")
        }
    }
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

    use super::{Budget, PingKind, handle_body, read_cause};

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

    fn budget(ms: u64, left: usize) -> Budget {
        Budget {
            deadline: soon(ms),
            left,
        }
    }

    #[tokio::test]
    async fn a_stalled_body_keeps_what_arrived_at_the_deadline() {
        let mut body = Body::new(Stalls(false));
        let mut b = budget(50, usize::MAX);
        let read = read_cause(&mut body, &mut b);
        let cause = tokio::time::timeout(Duration::from_secs(1), read)
            .await
            .expect("the read stops at the deadline");
        assert_eq!(cause.as_deref(), Some("Error: half"));
    }

    #[tokio::test]
    async fn the_cause_stops_reading_at_1_kb() {
        let frames = Arc::new(AtomicUsize::new(0));
        let mut body = Body::new(Long(frames.clone(), "xxxx"));
        let cause = read_cause(&mut body, &mut budget(60_000, usize::MAX)).await;
        assert_eq!(cause.map(|c| c.len()), Some(1024));
        assert_eq!(frames.load(Ordering::SeqCst), 256);
    }

    #[tokio::test]
    async fn both_reads_share_one_byte_cap() {
        let frames = Arc::new(AtomicUsize::new(0));
        let body = Body::new(Long(frames.clone(), "    "));
        let record = async |cause: Option<String>| (cause, frames.load(Ordering::SeqCst));
        let stored = handle_body(body, PingKind::Fail, budget(60_000, 10), record).await;
        assert_eq!(stored, (None, 3), "stored after 4 + 4 + 4 bytes reach 10");
        assert_eq!(
            frames.load(Ordering::SeqCst),
            3,
            "the drain has nothing left"
        );
    }

    #[tokio::test]
    async fn both_reads_share_one_deadline() {
        let body = Body::new(Stalls(false));
        let record = async |cause: Option<String>| cause;
        let handled = handle_body(body, PingKind::Fail, budget(50, usize::MAX), record);
        let cause = tokio::time::timeout(Duration::from_secs(1), handled)
            .await
            .expect("the drain stops at the same deadline");
        assert_eq!(cause.as_deref(), Some("Error: half"));
    }

    #[tokio::test]
    async fn an_ok_ping_is_stored_first_then_drains_its_body() {
        let frames = Arc::new(AtomicUsize::new(0));
        let body = Body::new(Long(frames.clone(), "xxxx"));
        let record = async |cause: Option<String>| (cause, frames.load(Ordering::SeqCst));
        let stored = handle_body(body, PingKind::Ok, budget(60_000, 10), record).await;
        assert_eq!(stored, (None, 0), "stored before a byte is read");
        assert_eq!(
            frames.load(Ordering::SeqCst),
            3,
            "drained until 4 + 4 + 4 bytes reach 10"
        );
    }
}
