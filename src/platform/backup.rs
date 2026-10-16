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
    if file.exists() {
        std::fs::remove_file(&file)?;
    }
    state.db.vacuum_into(&file).await?;
    remove_companions(&file)?;
    restrict(&file)?;
    prune(dir, state.config.backup_keep.max(1))?;
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

/// SEC-18: backups are readable by the service user only.
fn restrict(file: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(file, std::fs::Permissions::from_mode(0o600))?;
    }
    let _ = file;
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
