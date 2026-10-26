//! A user's pinned monitors and heartbeats in a project (decision Q48): they head their lists
//! and are drawn first on the Overview chart. A supervisor pins too; an API key never does.

use std::collections::HashSet;

use super::store::pins as store;
use crate::{
    incidents::SourceKind,
    platform::{caps, db::Db, now},
};

#[derive(Debug, thiserror::Error)]
pub enum PinError {
    /// The user already has `caps::PINS` of this kind pinned in the project.
    #[error("{max} {noun} are pinned in this project, which is the most, so unpin one to pin {name}.", max = caps::PINS)]
    Full { noun: &'static str, name: String },
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Pins or unpins one monitor or heartbeat of `project_id`, which the caller has found in that
/// project. Pinning a pinned one changes nothing. The cap is checked inside the write.
pub async fn set(
    db: &Db,
    user_id: i64,
    project_id: i64,
    kind: SourceKind,
    id: i64,
    name: &str,
    on: bool,
) -> Result<(), PinError> {
    let done = db
        .write(async |c| {
            if !on {
                store::remove(c, user_id, kind, id).await?;
                return Ok(true);
            }
            if !store::has(c, user_id, kind, id).await?
                && store::count(c, user_id, project_id, kind).await? >= caps::PINS
            {
                return Ok(false);
            }
            store::add(c, user_id, kind, id, now()).await?;
            Ok(true)
        })
        .await?;
    if done {
        return Ok(());
    }
    let noun = match kind {
        SourceKind::Monitor => "monitors",
        SourceKind::Heartbeat => "heartbeats",
    };
    Err(PinError::Full {
        noun,
        name: name.to_string(),
    })
}

/// The viewer's pinned ids of this kind in the project. A key principal (`None`) has none.
pub async fn ids(
    db: &Db,
    viewer: Option<i64>,
    project_id: i64,
    kind: SourceKind,
) -> anyhow::Result<HashSet<i64>> {
    match viewer {
        Some(user) => Ok(store::ids(&db.read().await?, user, project_id, kind)
            .await?
            .into_iter()
            .collect()),
        None => Ok(HashSet::new()),
    }
}
