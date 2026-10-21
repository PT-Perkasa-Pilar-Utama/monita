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
    let (done_tx, mut done_rx) = mpsc::unbounded_channel::<(i64, CheckResult)>();
    let auth = format!("Bearer {}", token.trim());
    tracing::info!(server = %server, concurrency, "probe started");

    let mut tick = tokio::time::interval(Duration::from_secs(1));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let stop = shutdown();
    tokio::pin!(stop);
    loop {
        tokio::select! {
            _ = &mut stop => break,
            _ = tick.tick() => {}
        }
        post_finished(&api, &server, &auth, &mut done_rx).await?;
        let free = slots.available_permits();
        if free == 0 {
            continue;
        }
        let jobs = match fetch(&api, &server, &auth, free).await {
            Ok(jobs) => jobs,
            Err(Fetch::Refused) => bail!(
                "The server refused the token. The location was deleted, or the token is wrong."
            ),
            Err(Fetch::Other(e)) => {
                tracing::warn!(error = %e, "could not reach the server; trying again in 5 s");
                tokio::time::sleep(BACKOFF).await;
                continue;
            }
        };
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
    post_finished(&api, &server, &auth, &mut done_rx).await
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
            "{server}/api/v1/probe/work?free={}",
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

/// Posts every finished result, in batches. A batch the server cannot take is dropped: the server
/// stamps a result with its arrival time, so a late result would be a wrong one.
async fn post_finished(
    api: &reqwest::Client,
    server: &str,
    auth: &str,
    done: &mut mpsc::UnboundedReceiver<(i64, CheckResult)>,
) -> anyhow::Result<()> {
    loop {
        let mut batch = Vec::new();
        while batch.len() < BATCH {
            let Ok((monitor_id, r)) = done.try_recv() else {
                break;
            };
            batch.push(Reported {
                monitor_id,
                ok: r.ok,
                status_code: r.status_code,
                latency_ms: r.latency_ms,
                error: r.error,
            });
        }
        if batch.is_empty() {
            return Ok(());
        }
        let sent = api
            .post(format!("{server}/api/v1/probe/results"))
            .header(reqwest::header::AUTHORIZATION, auth)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(serde_json::json!({ "results": batch }).to_string())
            .send()
            .await;
        match sent {
            Ok(r) if r.status() == reqwest::StatusCode::UNAUTHORIZED => {
                bail!(
                    "The server refused the token. The location was deleted, or the token is wrong."
                )
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
