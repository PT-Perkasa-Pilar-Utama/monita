//! Nightly backup (US-33): `VACUUM INTO` a dated file, keep the newest `MONITA_BACKUP_KEEP`.

use std::path::{Path, PathBuf};

use crate::app::AppState;

/// Writes `monita-YYYYMMDD.db` into the backup folder. `None` when no folder is set.
pub async fn run(state: &AppState, now: i64) -> anyhow::Result<Option<PathBuf>> {
    let Some(dir) = &state.config.backup_dir else {
        return Ok(None);
    };
    std::fs::create_dir_all(dir)?;
    let file = dir.join(format!(
        "monita-{}.db",
        super::time::day(now).replace('-', "")
    ));
    // SEC-18 during the copy too: VACUUM INTO creates the file with the process umask, so it is
    // written inside a private folder, restricted, then moved into place.
    let partial = dir.join(".partial");
    if partial.exists() {
        std::fs::remove_dir_all(&partial)?;
    }
    std::fs::create_dir(&partial)?;
    super::restrict(&partial, 0o700)?;
    let staged = partial.join("copy.db");
    state.db.vacuum_into(&staged).await?;
    remove_companions(&staged)?;
    super::restrict(&staged, 0o600)?;
    std::fs::rename(&staged, &file)?;
    std::fs::remove_dir(&partial)?;
    // The copy is good; a prune failure must not make the job redo it every 10 minutes.
    if let Err(e) = prune(dir, state.config.backup_keep.max(1)) {
        tracing::warn!(error = %e, "could not prune old backups");
    }
    tracing::info!(file = %file.display(), "backup written");
    Ok(Some(file))
}

/// `VACUUM INTO` can leave an empty `-wal` beside the copy; the copy is complete without it.
fn remove_companions(file: &Path) -> std::io::Result<()> {
    for suffix in ["-wal", "-shm"] {
        let companion = PathBuf::from(format!("{}{suffix}", file.display()));
        if companion.exists() {
            std::fs::remove_file(companion)?;
        }
    }
    Ok(())
}

/// Deletes all but the newest `keep` backups. The dated names sort by age.
fn prune(dir: &Path, keep: usize) -> std::io::Result<()> {
    let mut backups: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
                n.starts_with("monita-")
                    && n.ends_with(".db")
                    && n.len() == "monita-20261016.db".len()
            })
        })
        .collect();
    backups.sort();
    let excess = backups.len().saturating_sub(keep);
    for old in &backups[..excess] {
        std::fs::remove_file(old)?;
        remove_companions(old)?;
    }
    Ok(())
}
