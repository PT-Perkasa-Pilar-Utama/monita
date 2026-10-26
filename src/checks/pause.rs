//! Pause and resume (v1.1 Q38): a monitor or heartbeat a tech lead stopped for a while.

use super::store;
use crate::{
    incidents::SourceKind,
    platform::{db::Db, now},
};

/// Pauses or resumes a monitor or heartbeat (v1.1 Q38). A pause closes its open automatic
/// incident without an alert. A resumed monitor checks at once; a resumed heartbeat's deadline
/// starts over from now. False when it is not in the project; pausing twice changes nothing.
pub async fn set_paused(
    db: &Db,
    project_id: i64,
    kind: SourceKind,
    id: i64,
    paused: bool,
) -> anyhow::Result<bool> {
    db.write(async |c| set_paused_on(c, project_id, kind, id, paused, now()).await)
        .await
}

/// The project's paused monitors and heartbeats, for the Maintenance tab's count.
pub async fn paused_keys(
    db: &Db,
    project_id: i64,
) -> anyhow::Result<std::collections::HashSet<(SourceKind, i64)>> {
    store::source_keys(&db.read().await?, project_id, true).await
}

/// `set_paused` inside the caller's write, so a batch changes all of its sources or none.
pub(super) async fn set_paused_on(
    c: &crate::platform::sql::Connection,
    project_id: i64,
    kind: SourceKind,
    id: i64,
    paused: bool,
    t: i64,
) -> anyhow::Result<bool> {
    let (source, was) = match kind {
        SourceKind::Monitor => match store::find_monitor(c, project_id, id).await? {
            Some(m) => (m.source(), m.paused_at.is_some()),
            None => return Ok(false),
        },
        SourceKind::Heartbeat => match store::heartbeats::find(c, project_id, id).await? {
            Some(h) => (h.source(), h.paused_at.is_some()),
            None => return Ok(false),
        },
    };
    if was == paused {
        return Ok(true);
    }
    if paused {
        crate::incidents::source_paused(c, &source, t).await?;
    }
    match kind {
        SourceKind::Monitor => store::set_monitor_paused(c, id, paused, t).await?,
        SourceKind::Heartbeat => store::heartbeats::set_paused(c, id, paused, t).await?,
    }
    Ok(true)
}
