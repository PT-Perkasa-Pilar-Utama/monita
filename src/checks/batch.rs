//! One action over many monitors or heartbeats of a project: pause, resume or delete. A batch is
//! all or nothing: it runs in one write, and an id outside the project refuses the whole batch.

use super::{pause::set_paused_on, store};
use crate::{
    incidents::SourceKind,
    platform::{caps, db::Db, now},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Pause,
    Resume,
    Delete,
}

impl Action {
    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "pause" => Some(Self::Pause),
            "resume" => Some(Self::Resume),
            "delete" => Some(Self::Delete),
            _ => None,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BatchError {
    #[error("Pick at least one.")]
    Empty,
    #[error("Pick {0} or fewer at once.")]
    TooMany(i64),
    /// An id that is not a monitor or heartbeat of this project.
    #[error("One of them does not exist.")]
    Unknown,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Applies `action` to every id, in one write. Returns how many it covered. An open automatic
/// incident of a deleted source closes and sends its resolved alert, as a single delete does.
pub async fn apply(
    db: &Db,
    base_url: &str,
    project_id: i64,
    kind: SourceKind,
    ids: &[i64],
    action: Action,
) -> Result<usize, BatchError> {
    let mut ids = ids.to_vec();
    ids.sort_unstable();
    ids.dedup();
    let cap = match kind {
        SourceKind::Monitor => caps::MONITORS_PER_PROJECT,
        SourceKind::Heartbeat => caps::HEARTBEATS_PER_PROJECT,
    };
    if ids.is_empty() {
        return Err(BatchError::Empty);
    }
    if ids.len() as i64 > cap {
        return Err(BatchError::TooMany(cap));
    }
    let done = db
        .write(async |c| {
            let t = now();
            for &id in &ids {
                let found = match action {
                    Action::Pause => set_paused_on(c, project_id, kind, id, true, t).await?,
                    Action::Resume => set_paused_on(c, project_id, kind, id, false, t).await?,
                    Action::Delete => delete_on(c, base_url, project_id, kind, id, t).await?,
                };
                // Returning an error rolls the whole write back.
                if !found {
                    return Err(anyhow::Error::new(BatchError::Unknown));
                }
            }
            Ok(ids.len())
        })
        .await;
    done.map_err(|e| match e.downcast::<BatchError>() {
        Ok(b) => b,
        Err(e) => BatchError::Other(e),
    })
}

/// Deletes one source inside the caller's write. False when it is not in the project.
pub(super) async fn delete_on(
    c: &turso::Connection,
    base_url: &str,
    project_id: i64,
    kind: SourceKind,
    id: i64,
    t: i64,
) -> anyhow::Result<bool> {
    let source = match kind {
        SourceKind::Monitor => store::find_monitor(c, project_id, id)
            .await?
            .map(|m| m.source()),
        SourceKind::Heartbeat => store::heartbeats::find(c, project_id, id)
            .await?
            .map(|h| h.source()),
    };
    let Some(source) = source else {
        return Ok(false);
    };
    if let Some(ev) = crate::incidents::source_deleted(c, &source, t).await? {
        crate::notify::send(c, &ev, base_url).await?;
    }
    match kind {
        SourceKind::Monitor => store::delete_monitor(c, project_id, id).await?,
        SourceKind::Heartbeat => store::heartbeats::delete(c, project_id, id).await?,
    }
    Ok(true)
}
