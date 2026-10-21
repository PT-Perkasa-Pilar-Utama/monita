//! Checks from more than one place (v1.1 Q26, technical-specs 14): the locations a super admin
//! adds, the work a location's probe claims, and the verdict each new result feeds the incident
//! engine.

use super::verdict::{self, Place, THIS_SERVER, Verdict};
use super::{CheckResult, Job, Monitor, store};
use crate::platform::{caps, db::Db, now, token};

pub use store::locations::LocationRow;

/// Location tokens start with this, as API keys start with `mk_`.
pub const TOKEN_PREFIX: &str = "ml_";

#[derive(Debug, thiserror::Error)]
pub enum LocationError {
    #[error("Enter a name of 1 to 40 characters.")]
    BadName,
    #[error("A location already has this name.")]
    Taken,
    #[error(
        "Monita takes {} locations at most. Delete one first.",
        caps::LOCATIONS
    )]
    Full,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub async fn list(db: &Db) -> anyhow::Result<Vec<LocationRow>> {
    store::locations::list(&db.read().await?).await
}

/// Every location as `(id, name)`, for the monitor form.
pub async fn names(db: &Db) -> anyhow::Result<Vec<(i64, String)>> {
    store::locations::names(&db.read().await?).await
}

/// Adds a location. Returns its id and its token, which is shown once and stored only as a hash.
pub async fn create(db: &Db, name: &str) -> Result<(i64, String), LocationError> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 40 {
        return Err(LocationError::BadName);
    }
    let token = format!("{TOKEN_PREFIX}{}", token::random_token());
    let prefix = token[TOKEN_PREFIX.len()..TOKEN_PREFIX.len() + 4].to_string();
    let hash = token::hash(&token);
    let made = db
        .write(async |c| {
            if store::locations::count(c).await? >= caps::LOCATIONS {
                return Ok(Err(LocationError::Full));
            }
            if store::locations::name_taken(c, name).await? {
                return Ok(Err(LocationError::Taken));
            }
            Ok(Ok(
                store::locations::insert(c, name, &hash, &prefix, now()).await?
            ))
        })
        .await??;
    Ok((made, token))
}

/// Deletes a location with its assignments and results. False when there was none.
pub async fn delete(db: &Db, id: i64) -> anyhow::Result<bool> {
    db.write(async |c| store::locations::delete(c, id).await)
        .await
}

/// The location a token belongs to, and when it last called. None for an unknown token.
pub async fn authenticate(
    c: &turso::Connection,
    raw: &str,
) -> anyhow::Result<Option<(i64, Option<i64>)>> {
    store::locations::by_token(c, &token::hash(raw.trim())).await
}

/// Claims up to `free` (capped at `PROBE_BATCH`) of the location's due checks and returns them as
/// jobs with their secrets. Takes the writer only when something is due, or to note the probe's
/// call once every 30 seconds.
pub async fn claim_work(
    db: &Db,
    location_id: i64,
    last_seen_at: Option<i64>,
    free: i64,
) -> anyhow::Result<Vec<Job>> {
    let t = now();
    let due = store::locations::any_due(&db.read().await?, location_id, t).await?;
    if !due {
        if last_seen_at.is_none_or(|at| t - at >= 30) {
            db.write(async |c| store::locations::touch(c, location_id, t).await)
                .await?;
        }
        return Ok(Vec::new());
    }
    let limit = free.clamp(1, caps::PROBE_BATCH);
    db.write(async |c| {
        store::locations::touch(c, location_id, t).await?;
        let mut jobs = Vec::new();
        for m in store::claim_for_location(c, location_id, t, limit).await? {
            let secret = Job::read_secret(c, &m).await?;
            jobs.push(Job::new(&m, secret));
        }
        Ok(jobs)
    })
    .await
}

/// One result a probe reports. The server stamps the time.
#[derive(serde::Deserialize)]
pub struct Reported {
    pub monitor_id: i64,
    pub ok: bool,
    pub status_code: Option<u16>,
    pub latency_ms: i64,
    pub error: Option<String>,
}

/// Stores and judges each result of a location's batch. Returns (stored, skipped): a result for a
/// monitor the location does not check is skipped.
pub async fn post_results(
    db: &Db,
    base_url: &str,
    location_id: i64,
    results: Vec<Reported>,
) -> anyhow::Result<(usize, usize)> {
    let (mut stored, mut skipped) = (0, 0);
    for r in results {
        let monitor = {
            let c = db.read().await?;
            if store::locations::assigned(&c, r.monitor_id, location_id).await? {
                store::find_any(&c, r.monitor_id).await?
            } else {
                None
            }
        };
        let Some(monitor) = monitor else {
            skipped += 1;
            continue;
        };
        let result = CheckResult {
            checked_at: now(),
            ok: r.ok,
            status_code: r.status_code,
            latency_ms: r.latency_ms.clamp(0, 600_000),
            error: r.error.map(|e| cut(&e, 1024)),
            cert_expires_at: None,
        };
        record_at(db, base_url, &monitor, Some(location_id), &result).await?;
        stored += 1;
    }
    db.write(async |c| store::locations::touch(c, location_id, now()).await)
        .await?;
    Ok((stored, skipped))
}

/// Stores one result from a place and feeds the monitor's verdict to the incident engine, in one
/// transaction (spec 12). `location_id` None is this server, whose HTTPS checks also keep the
/// certificate's expiry.
pub async fn record_at(
    db: &Db,
    base_url: &str,
    monitor: &Monitor,
    location_id: Option<i64>,
    result: &CheckResult,
) -> anyhow::Result<()> {
    db.write(async |c| {
        store::insert_check(c, monitor.id, location_id, result).await?;
        let signal = signal(c, monitor, result).await?;
        if let Some(ev) =
            crate::incidents::observe(c, &monitor.source(), &signal, result.checked_at).await?
        {
            crate::notify::send(c, &ev, base_url).await?;
        }
        if let (None, Some(expires_at)) = (location_id, result.cert_expires_at) {
            super::record_cert(c, base_url, monitor, expires_at).await?;
        }
        Ok(())
    })
    .await
}

/// What the new result means for the monitor. A monitor with no locations reads it directly, as
/// before 1.1, without a query.
async fn signal(
    c: &turso::Connection,
    monitor: &Monitor,
    result: &CheckResult,
) -> anyhow::Result<crate::incidents::Signal> {
    use crate::incidents::Signal;
    let located = store::locations::of_monitor(c, monitor.id).await?;
    let verdict = if located.is_empty() {
        if result.ok {
            Verdict::Passed
        } else {
            Verdict::Failed {
                cause: result
                    .error
                    .clone()
                    .unwrap_or_else(|| "Check failed".into()),
            }
        }
    } else {
        let newest = store::locations::newest_by_place(c, monitor.id).await?;
        let find = |id: Option<i64>| {
            newest
                .iter()
                .find(|n| n.0 == id)
                .map(|n| (n.1, n.2, n.3.as_deref()))
        };
        let mut places = vec![Place {
            name: THIS_SERVER,
            newest: find(None),
        }];
        places.extend(located.iter().map(|(id, name)| Place {
            name,
            newest: find(Some(*id)),
        }));
        verdict::judge(&places, monitor.interval_s, monitor.quorum, now())
    };
    Ok(match verdict {
        Verdict::Passed => Signal::CheckOk,
        Verdict::Failed { cause } => Signal::CheckFailed { cause },
    })
}

/// A place's newest result for the monitor page: the location, or None for this server.
#[derive(serde::Serialize)]
pub struct PlaceView {
    pub location_id: Option<i64>,
    pub name: String,
    /// `(checked_at, ok, error)`.
    pub last: Option<(i64, bool, Option<String>)>,
}

/// The monitor's locations and each place's newest result, this server first. Empty when the
/// monitor checks from this server only.
pub async fn places(db: &Db, monitor_id: i64) -> anyhow::Result<(Vec<i64>, Vec<PlaceView>)> {
    let c = db.read().await?;
    let located = store::locations::of_monitor(&c, monitor_id).await?;
    if located.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }
    let newest = store::locations::newest_by_place(&c, monitor_id).await?;
    let last = |id: Option<i64>| {
        newest
            .iter()
            .find(|n| n.0 == id)
            .map(|n| (n.1, n.2, n.3.clone()))
    };
    let mut out = vec![PlaceView {
        location_id: None,
        name: THIS_SERVER.into(),
        last: last(None),
    }];
    out.extend(located.iter().map(|(id, name)| PlaceView {
        location_id: Some(*id),
        name: name.clone(),
        last: last(Some(*id)),
    }));
    Ok((located.iter().map(|(id, _)| *id).collect(), out))
}

/// The first `max` bytes of `s`, cut on a character boundary.
fn cut(s: &str, max: usize) -> String {
    let mut end = s.len().min(max);
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    s[..end].to_string()
}
