//! `monita probe`: runs checks for one location of another Monita and reports them
//! (technical-specs 14, api-specs 10 section 10.5). No env file, no database.

use std::{sync::Arc, time::Duration};

use anyhow::{Context, bail};
use serde::Deserialize;
use tokio::sync::{Semaphore, mpsc};

use crate::checks::{CheckResult, Clients, Job};

/// One check at a time per slot; the server hands out at most this many jobs per call anyway.
pub const MAX_CONCURRENCY: u16 = 50;
/// Results posted in one call (`caps::PROBE_BATCH`).
const BATCH: usize = 50;
/// How long the server may hold a poll for work while nothing is due (`caps::PROBE_WAIT_S`).
const WAIT_S: u64 = crate::platform::caps::PROBE_WAIT_S;
/// Wait after the server could not be reached.
const BACKOFF: Duration = Duration::from_secs(5);

#[derive(Deserialize)]
struct Envelope<T> {
    data: T,
}

#[derive(Deserialize)]
struct Work {
    jobs: Vec<Job>,
}

/// What a probe sends for one check. The server stamps the time.
#[derive(serde::Serialize)]
struct Reported {
    monitor_id: i64,
    ok: bool,
    status_code: Option<u16>,
    latency_ms: i64,
    error: Option<String>,
}

pub async fn run(server: &str, token: &str, concurrency: u16) -> anyhow::Result<()> {
    let server = server.trim().trim_end_matches('/').to_string();
    if !crate::platform::valid_http_url(&server) {
        bail!("--server must be a URL such as https://monita.company.co.id");
    }
    if !token
        .trim()
        .starts_with(crate::checks::locations::TOKEN_PREFIX)
    {
        bail!("The token starts with ml_. Copy it from the Locations page.");
    }
    let api = reqwest::Client::builder()
        .user_agent(concat!("Monita-probe/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(30))
        .build()?;
    let clients = Clients::new()?;
    let slots = Arc::new(Semaphore::new(usize::from(
        concurrency.clamp(1, MAX_CONCURRENCY),
    )));
    let (done_tx, done_rx) = mpsc::unbounded_channel::<(i64, CheckResult)>();
    let auth = format!("Bearer {}", token.trim());
    tracing::info!(server = %server, concurrency, "probe started");

    // Results go out from their own task as checks finish, so a held poll never delays them.
    let (refused_tx, mut refused) = tokio::sync::watch::channel(false);
    let poster = tokio::spawn(post_loop(
        api.clone(),
        server.clone(),
        auth.clone(),
        done_rx,
        refused_tx,
    ));
    let stop = shutdown();
    tokio::pin!(stop);
    let refused_msg =
        "The server refused the token, because the location was deleted or the token is wrong.";
    loop {
        // Ask only with a free slot, so every job handed out starts at once.
        tokio::select! {
            _ = &mut stop => break,
            _ = refused.wait_for(|r| *r) => bail!(refused_msg),
            permit = slots.clone().acquire_owned() => drop(permit),
        }
        let free = slots.available_permits();
        let asked = tokio::time::Instant::now();
        let fetched = tokio::select! {
            _ = &mut stop => break,
            _ = refused.wait_for(|r| *r) => bail!(refused_msg),
            f = fetch(&api, &server, &auth, free) => f,
        };
        let jobs = match fetched {
            Ok(jobs) => jobs,
            Err(Fetch::Refused) => bail!(refused_msg),
            Err(Fetch::Other(e)) => {
                tracing::warn!(error = %e, "could not reach the server; trying again in 5 s");
                tokio::time::sleep(BACKOFF).await;
                continue;
            }
        };
        // A server from before long polling answers at once: then wait a second, as before.
        if jobs.is_empty() && asked.elapsed() < Duration::from_secs(1) {
            tokio::select! {
                _ = &mut stop => break,
                _ = tokio::time::sleep(Duration::from_secs(1)) => {}
            }
        }
        for job in jobs {
            let Ok(permit) = slots.clone().try_acquire_owned() else {
                break;
            };
            let (clients, done) = (clients.clone(), done_tx.clone());
            tokio::spawn(async move {
                let result = job.run(&clients).await;
                drop(permit);
                let _ = done.send((job.monitor_id, result));
            });
        }
    }
    tracing::info!("stopping: finishing the checks in flight");
    let all = u32::from(concurrency.clamp(1, MAX_CONCURRENCY));
    let _ = tokio::time::timeout(Duration::from_secs(35), slots.acquire_many(all)).await;
    // The poster ends once every sender is gone and the last results are out.
    drop(done_tx);
    let _ = tokio::time::timeout(Duration::from_secs(35), poster).await;
    Ok(())
}

enum Fetch {
    Refused,
    Other(anyhow::Error),
}

async fn fetch(
    api: &reqwest::Client,
    server: &str,
    auth: &str,
    free: usize,
) -> Result<Vec<Job>, Fetch> {
    let res = api
        .get(format!(
            "{server}/api/v1/probe/work?free={}&wait={WAIT_S}",
            free.min(BATCH)
        ))
        .header(reqwest::header::AUTHORIZATION, auth)
        .send()
        .await
        .map_err(|e| Fetch::Other(e.without_url().into()))?;
    if res.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(Fetch::Refused);
    }
    if !res.status().is_success() {
        return Err(Fetch::Other(anyhow::anyhow!(
            "work answered {}",
            res.status()
        )));
    }
    let bytes = res
        .bytes()
        .await
        .map_err(|e| Fetch::Other(e.without_url().into()))?;
    let body: Envelope<Work> = serde_json::from_slice(&bytes)
        .context("work answered with an unexpected body")
        .map_err(Fetch::Other)?;
    Ok(body.data.jobs)
}

/// Posts results as checks finish, up to `BATCH` a call, until every sender is gone. A batch the
/// server cannot take is dropped: the server stamps a result with its arrival time, so a late
/// result would be a wrong one. A refused token flips `refused`, which stops the probe.
async fn post_loop(
    api: reqwest::Client,
    server: String,
    auth: String,
    mut done: mpsc::UnboundedReceiver<(i64, CheckResult)>,
    refused: tokio::sync::watch::Sender<bool>,
) {
    let report = |(monitor_id, r): (i64, CheckResult)| Reported {
        monitor_id,
        ok: r.ok,
        status_code: r.status_code,
        latency_ms: r.latency_ms,
        error: r.error,
    };
    while let Some(first) = done.recv().await {
        let mut batch = vec![report(first)];
        while batch.len() < BATCH {
            match done.try_recv() {
                Ok(next) => batch.push(report(next)),
                Err(_) => break,
            }
        }
        let sent = api
            .post(format!("{server}/api/v1/probe/results"))
            .header(reqwest::header::AUTHORIZATION, &auth)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(serde_json::json!({ "results": batch }).to_string())
            .send()
            .await;
        match sent {
            Ok(r) if r.status() == reqwest::StatusCode::UNAUTHORIZED => {
                refused.send_replace(true);
                return;
            }
            Ok(r) if r.status().is_success() => {}
            Ok(r) => tracing::warn!(
                status = r.status().as_u16(),
                n = batch.len(),
                "results refused; dropped"
            ),
            Err(e) => {
                tracing::warn!(error = %e.without_url(), n = batch.len(), "could not post results; dropped")
            }
        }
    }
}

/// Resolves on Ctrl-C, or SIGTERM on Unix.
async fn shutdown() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    {
        let mut term =
            match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
                Ok(s) => s,
                Err(_) => return ctrl_c.await,
            };
        tokio::select! {
            _ = ctrl_c => {}
            _ = term.recv() => {}
        }
    }
    #[cfg(not(unix))]
    ctrl_c.await;
}
