//! Database handle. Spec: docs/technical-specs/05-module-definitions.md section 5.8.5.
//! SQLite through rusqlite (`platform::sql`). All writes go through one writer connection under a
//! lock, so a write waits its turn instead of meeting SQLITE_BUSY; reads use pooled connections,
//! which WAL lets run beside the writer.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use anyhow::Context;
use tokio::sync::Mutex;

use super::sql::{Connection, Pool};

/// Idle reading connections kept for the next request. More can be open at once; the rest close.
const IDLE_READERS: usize = 8;
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);
/// `PRAGMA auto_vacuum` answers 2 for incremental.
const INCREMENTAL: i64 = 2;
/// 2000 pages of 4 KiB, 8 MB: about 50 ms of the writer on a 350 MB file.
const FREE_BATCH: i64 = 2000;

#[derive(Clone)]
pub struct Db(Arc<Inner>);

struct Inner {
    path: PathBuf,
    writer: Mutex<Writer>,
    readers: Arc<Pool>,
    /// Held while the process runs: a second Monita process on the same database stops at open,
    /// as two schedulers on one database would send every alert twice.
    _lock: std::fs::File,
}

struct Writer {
    conn: Connection,
    /// True from just before `PRAGMA foreign_keys = OFF` until `clean` turns the checks back on,
    /// so a `write_without_fk` dropped at any point still leaves foreign keys on for the next user.
    fk_off: bool,
}

impl Writer {
    /// Undoes what a dropped caller future left behind. A client that disconnects mid-request
    /// drops its handler, and with it a `write` at any await point, BEGIN included. A ping runs
    /// in its own task, so its `write` is not dropped this way. The connection itself says
    /// whether a transaction is open, so no flag can miss one.
    async fn clean(&mut self, path: &Path) {
        let open = match self.conn.is_autocommit() {
            Ok(autocommit) => !autocommit,
            Err(e) => {
                tracing::error!(error = %e, "could not read the writer's transaction state");
                true
            }
        };
        let mut stuck = false;
        if open {
            tracing::warn!("rolling back a transaction left open by a dropped write");
            if let Err(e) = self.conn.execute("ROLLBACK", ()).await {
                tracing::error!(error = %e, "rollback of an abandoned transaction failed");
                stuck = true;
            }
        }
        // The pragma is a silent no-op inside a transaction, so `fk_off` clears only once the
        // connection is out of one and the pragma ran.
        if self.fk_off {
            let restored = self.conn.is_autocommit().unwrap_or(false)
                && self
                    .conn
                    .execute("PRAGMA foreign_keys = ON", ())
                    .await
                    .inspect_err(
                        |e| tracing::error!(error = %e, "could not turn foreign keys back on"),
                    )
                    .is_ok();
            if restored {
                self.fk_off = false;
            } else {
                stuck = true;
            }
        }
        // A connection that cannot be put back is replaced, and the new one starts with foreign keys
        // on, so writes go on instead of failing forever.
        if stuck {
            match connect(path, true) {
                Ok(conn) => {
                    tracing::warn!("replaced the writer connection after a failed cleanup");
                    self.conn = conn;
                    self.fk_off = false;
                }
                Err(e) => tracing::error!(error = %e, "could not replace the writer connection"),
            }
        }
    }
}

impl Db {
    pub async fn open(path: &Path) -> anyhow::Result<Self> {
        if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            std::fs::create_dir_all(dir)?;
        }
        anyhow::ensure!(path.to_str().is_some(), "database path must be UTF-8");
        let lock_path = PathBuf::from(format!("{}.lock", path.display()));
        let mut lock = std::fs::OpenOptions::new();
        lock.create(true).truncate(false).write(true);
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::custom_flags(&mut lock, libc::O_NOFOLLOW);
        let lock = lock
            .open(&lock_path)
            .with_context(|| format!("opening {}", lock_path.display()))?;
        if lock.try_lock().is_err() {
            anyhow::bail!(
                "{} is open in another Monita process; stop that process first",
                path.display()
            );
        }
        let conn = connect(path, true)?;
        // SEC-18: the database holds secrets, so only its owner reads it, whatever the umask.
        for companion in ["", "-wal", "-shm", ".lock"] {
            let f = PathBuf::from(format!("{}{companion}", path.display()));
            if f.exists() {
                super::restrict(&f, 0o600)
                    .with_context(|| format!("restricting {}", f.display()))?;
            }
        }
        Ok(Self(Arc::new(Inner {
            path: path.to_path_buf(),
            writer: Mutex::new(Writer {
                conn,
                fk_off: false,
            }),
            readers: Arc::new(Pool::new(IDLE_READERS)),
            _lock: lock,
        })))
    }

    /// A reading connection from the pool. It skips the `foreign_keys` pragma: only the writer
    /// needs it.
    pub async fn read(&self) -> anyhow::Result<Connection> {
        let conn = match self.0.readers.take() {
            Some(c) => c,
            None => open_raw(&self.0.path, false)?,
        };
        Ok(Connection::pooled(conn, self.0.readers.clone()))
    }

    /// Runs `f` on the writer inside `BEGIN IMMEDIATE ... COMMIT`. Rolls back when `f` fails.
    pub async fn write<T>(
        &self,
        f: impl AsyncFnOnce(&Connection) -> anyhow::Result<T>,
    ) -> anyhow::Result<T> {
        self.write_inner(false, f).await
    }

    /// `write` with foreign keys off, for table rebuilds (SQLite's 12-step ALTER procedure) and
    /// reset-state. The pragma cannot change inside a transaction, so it wraps BEGIN and COMMIT.
    /// Nothing checks the references afterwards, so `f` must keep every one valid itself.
    pub async fn write_without_fk<T>(
        &self,
        f: impl AsyncFnOnce(&Connection) -> anyhow::Result<T>,
    ) -> anyhow::Result<T> {
        self.write_inner(true, f).await
    }

    async fn write_inner<T>(
        &self,
        fk_off: bool,
        f: impl AsyncFnOnce(&Connection) -> anyhow::Result<T>,
    ) -> anyhow::Result<T> {
        let mut w = self.0.writer.lock().await;
        w.clean(&self.0.path).await;
        anyhow::ensure!(
            fk_off || !w.fk_off,
            "foreign keys could not be turned back on; refusing to write without them"
        );
        // Each flag is set before its statement: a future dropped while the statement runs must
        // still leave `clean` a flag to undo. A statement that fails clears its flag again.
        if fk_off {
            w.fk_off = true;
            if let Err(e) = w.conn.execute("PRAGMA foreign_keys = OFF", ()).await {
                w.fk_off = false;
                return Err(e.into());
            }
        }
        let result = Self::transaction(&mut w, f).await;
        w.clean(&self.0.path).await;
        result
    }

    async fn transaction<T>(
        w: &mut Writer,
        f: impl AsyncFnOnce(&Connection) -> anyhow::Result<T>,
    ) -> anyhow::Result<T> {
        w.conn.execute("BEGIN IMMEDIATE", ()).await?;
        let result = match f(&w.conn).await {
            Ok(v) => match w.conn.execute("COMMIT", ()).await {
                Ok(_) => return Ok(v),
                Err(e) => Err(e.into()),
            },
            Err(e) => Err(e),
        };
        if let Err(rb) = w.conn.execute("ROLLBACK", ()).await {
            tracing::error!(error = %rb, "rollback failed");
        }
        result
    }

    /// Writes a consistent copy of the database to `path` (US-33). It runs on its own connection
    /// on a blocking thread, so writes and requests go on during a long copy. It refuses to
    /// overwrite: remove `path` first.
    pub async fn vacuum_into(&self, path: &Path) -> anyhow::Result<()> {
        let target = path
            .to_str()
            .context("backup path must be UTF-8")?
            .replace('\'', "''");
        let source = self.0.path.clone();
        tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            connect(&source, false)?
                .with(|c| c.execute_batch(&format!("VACUUM INTO '{target}'")))?;
            Ok(())
        })
        .await?
    }

    /// Switches a file made before v1.1 to incremental auto-vacuum, so `free_pages` can give
    /// deleted space back to the disk. A new file has it from the start (`open_raw`). The switch
    /// rewrites the file once: 4 s for 350 MB, and it needs that much free disk for the copy. It
    /// runs on the writer, the one connection that must know the new mode, and blocks its thread,
    /// so it runs at start, before the server listens. Answers whether it switched.
    pub async fn enable_incremental_vacuum(&self) -> anyhow::Result<bool> {
        let mut w = self.0.writer.lock().await;
        w.clean(&self.0.path).await;
        if query_i64(&w.conn, "PRAGMA auto_vacuum", ()).await? == Some(INCREMENTAL) {
            return Ok(false);
        }
        w.conn
            .execute_batch("PRAGMA auto_vacuum = INCREMENTAL; VACUUM")
            .await?;
        Ok(true)
    }

    /// Gives free pages back to the disk, `FREE_BATCH` per write, so a check or a ping waits for
    /// one batch at most. It does nothing on a file without incremental auto-vacuum.
    pub async fn free_pages(&self) -> anyhow::Result<()> {
        let mut before = i64::MAX;
        loop {
            let left = self
                .write(async |c| {
                    if query_i64(c, "PRAGMA auto_vacuum", ()).await? != Some(INCREMENTAL) {
                        return Ok(0);
                    }
                    c.execute_batch(format!("PRAGMA incremental_vacuum({FREE_BATCH})"))
                        .await?;
                    Ok(query_i64(c, "PRAGMA freelist_count", ())
                        .await?
                        .unwrap_or(0))
                })
                .await?;
            if left == 0 || left >= before {
                return Ok(());
            }
            before = left;
        }
    }

    /// Drops every table. Only reset-state calls this.
    pub async fn drop_all_tables(&self) -> anyhow::Result<()> {
        let names = query_all(
            &self.read().await?,
            "SELECT name FROM sqlite_schema WHERE type = 'table' AND name NOT LIKE 'sqlite_%' LIMIT 100",
            (),
            |r| r.get::<String>(0),
        )
        .await?;
        self.write_without_fk(async |c| {
            for name in &names {
                c.execute(
                    &format!("DROP TABLE IF EXISTS \"{}\"", name.replace('"', "\"\"")),
                    (),
                )
                .await?;
            }
            Ok(())
        })
        .await
    }
}

/// A new connection with the settings every connection needs. The writer also turns on WAL,
/// full sync on commit, and foreign keys.
fn connect(path: &Path, writer: bool) -> anyhow::Result<Connection> {
    Ok(Connection::new(open_raw(path, writer)?))
}

fn open_raw(path: &Path, writer: bool) -> anyhow::Result<rusqlite::Connection> {
    let conn = rusqlite::Connection::open(path)
        .with_context(|| format!("opening the database {}", path.display()))?;
    conn.busy_timeout(BUSY_TIMEOUT)?;
    if writer {
        // Takes effect only on a file with no tables yet; an older file switches in
        // `enable_incremental_vacuum`.
        conn.execute_batch("PRAGMA auto_vacuum = INCREMENTAL")?;
        let mode: String = conn.query_row("PRAGMA journal_mode = WAL", [], |r| r.get(0))?;
        anyhow::ensure!(
            mode.eq_ignore_ascii_case("wal"),
            "the database refused WAL mode: {mode}"
        );
        conn.execute_batch("PRAGMA synchronous = FULL; PRAGMA foreign_keys = ON;")?;
    }
    Ok(conn)
}

/// Runs a query and maps every row with `f`.
pub async fn query_all<T>(
    conn: &Connection,
    sql: impl AsRef<str>,
    params: impl super::sql::IntoParams,
    f: impl Fn(&super::sql::Row) -> super::sql::Result<T>,
) -> anyhow::Result<Vec<T>> {
    let mut rows = conn.query(sql.as_ref(), params).await?;
    let mut out = Vec::new();
    while let Some(r) = rows.next().await? {
        out.push(f(&r)?);
    }
    Ok(out)
}

/// Runs a query that returns one integer, such as `SELECT count(*) ...`.
pub async fn query_i64(
    conn: &Connection,
    sql: &str,
    params: impl super::sql::IntoParams,
) -> anyhow::Result<Option<i64>> {
    let mut rows = conn.query(sql, params).await?;
    match rows.next().await? {
        Some(row) => Ok(row.get::<Option<i64>>(0)?),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests;
