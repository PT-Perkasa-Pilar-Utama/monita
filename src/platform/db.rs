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
        let lock = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
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
                super::restrict(&f, 0o600)?;
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
mod tests {
    use super::*;

    #[tokio::test]
    async fn foreign_keys_are_back_on_after_a_dropped_write_without_them() {
        let dir = tempfile::tempdir().unwrap();
        let db = Db::open(&dir.path().join("t.db")).await.unwrap();
        crate::platform::migrate::run(&db).await.unwrap();
        let abandoned = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            db.write_without_fk(async |_| {
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                Ok(())
            }),
        )
        .await;
        assert!(abandoned.is_err(), "the write was dropped by the timeout");
        db.write(async |c| {
            c.execute("INSERT INTO settings (key, value) VALUES ('ok', '1')", ())
                .await?;
            Ok(())
        })
        .await
        .expect("an ordinary write works");
        let orphan = db
            .write(async |c| {
                c.execute(
                    "INSERT INTO check_results (monitor_id, checked_at, ok) VALUES (999, 1, 1)",
                    (),
                )
                .await?;
                Ok(())
            })
            .await;
        assert!(
            orphan.is_err(),
            "a row for a missing monitor is refused again"
        );
    }

    #[tokio::test]
    async fn open_creates_the_missing_data_folder() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data/nested/t.db");
        Db::open(&path).await.unwrap();
        assert!(path.parent().unwrap().is_dir());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_write_does_not_wait_for_a_backup_copy() {
        let dir = tempfile::tempdir().unwrap();
        let db = Db::open(&dir.path().join("t.db")).await.unwrap();
        db.write(async |c| {
            c.execute("CREATE TABLE big (v TEXT)", ()).await?;
            // 40 MB: SQLite copies a few MB within the sleep below, and the copy must still run
            // when the write lands.
            for i in 0..40_000 {
                c.execute("INSERT INTO big VALUES (?1)", [format!("{i:0>1000}")])
                    .await?;
            }
            Ok(())
        })
        .await
        .unwrap();
        let copy = {
            let (db, path) = (db.clone(), dir.path().join("copy.db"));
            tokio::spawn(async move { db.vacuum_into(&path).await })
        };
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        db.write(async |c| {
            c.execute("INSERT INTO big VALUES ('during')", ()).await?;
            Ok(())
        })
        .await
        .unwrap();
        assert!(
            !copy.is_finished(),
            "the write finished while the copy still ran"
        );
        copy.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn drop_all_tables_then_migrate_again() {
        let dir = tempfile::tempdir().unwrap();
        let db = Db::open(&dir.path().join("t.db")).await.unwrap();
        crate::platform::migrate::run(&db).await.unwrap();
        db.drop_all_tables().await.unwrap();
        crate::platform::migrate::run(&db).await.unwrap();
        let conn = db.read().await.unwrap();
        let n = query_i64(&conn, "SELECT count(*) FROM settings", ())
            .await
            .unwrap();
        assert_eq!(n, Some(crate::platform::settings::KEYS.len() as i64));
    }

    #[tokio::test]
    async fn a_write_dropped_mid_transaction_is_rolled_back_before_the_next() {
        let dir = tempfile::tempdir().unwrap();
        let db = Db::open(&dir.path().join("t.db")).await.unwrap();
        crate::platform::migrate::run(&db).await.unwrap();
        let abandoned = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            db.write(async |c| {
                c.execute(
                    "INSERT INTO settings (key, value) VALUES ('ghost', '1')",
                    (),
                )
                .await?;
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                Ok(())
            }),
        )
        .await;
        assert!(abandoned.is_err(), "the write was dropped by the timeout");
        db.write(async |c| {
            c.execute("INSERT INTO settings (key, value) VALUES ('next', '1')", ())
                .await?;
            Ok(())
        })
        .await
        .expect("the next write works");
        let conn = db.read().await.unwrap();
        assert_eq!(
            query_i64(
                &conn,
                "SELECT count(*) FROM settings WHERE key = 'ghost'",
                ()
            )
            .await
            .unwrap(),
            Some(0)
        );
        assert_eq!(
            query_i64(
                &conn,
                "SELECT count(*) FROM settings WHERE key = 'next'",
                ()
            )
            .await
            .unwrap(),
            Some(1)
        );
    }
}
