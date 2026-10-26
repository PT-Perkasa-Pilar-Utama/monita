//! A user's pinned projects (decision Q48): they head the project list and the projects chart.
//! A pin is a personal setting, so a supervisor sets one too; an API key never does.

use super::{Project, store::pins as store};
use crate::platform::{caps, db::Db, now};

#[derive(Debug, thiserror::Error)]
pub enum PinError {
    /// The user already has `caps::PINS` pinned; the text names the one they tried.
    #[error("{max} projects are pinned, which is the most, so unpin one to pin {0}.", max = caps::PINS)]
    Full(String),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Pins or unpins `project` for the user. Pinning a pinned project changes nothing. The cap is
/// checked inside the write, so two pins at once cannot pass it together.
pub async fn set(db: &Db, user_id: i64, project: &Project, on: bool) -> Result<(), PinError> {
    let done = db
        .write(async |c| {
            if !on {
                store::remove(c, user_id, project.id).await?;
                return Ok(true);
            }
            if !store::has(c, user_id, project.id).await?
                && store::count(c, user_id).await? >= caps::PINS
            {
                return Ok(false);
            }
            store::add(c, user_id, project.id, now()).await?;
            Ok(true)
        })
        .await?;
    if done {
        Ok(())
    } else {
        Err(PinError::Full(project.name.clone()))
    }
}

/// The viewer's pinned project ids. A key principal (`None`) has none.
pub async fn ids(db: &Db, viewer: Option<i64>) -> anyhow::Result<Vec<i64>> {
    match viewer {
        Some(id) => store::ids(&db.read().await?, id).await,
        None => Ok(Vec::new()),
    }
}
