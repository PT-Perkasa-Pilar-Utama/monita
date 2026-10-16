//! Database handle. Spec: docs/technical-specs/05-module-definitions.md section 5.8.5.
//! All writes go through one writer connection: turso returns "busy" instead of waiting,
//! and the spike measured one mutex-guarded writer 11x faster than many connections.

use std::{path::Path, sync::Arc};

use anyhow::Context;
use tokio::sync::Mutex;
use turso::{Builder, Connection, Database};

#[derive(Clone)]
pub struct Db {
    database: Database,
    writer: Arc<Mutex<Writer>>,
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
    async fn clean(&mut self, database: &Database) {
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
        // on, so writes go on instead of failing forever. turso 0.8.3's `Connection` has no
        // `close`; dropping it is the only way to release it.
        if stuck {
            match connect(database).await {
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
        let path = path.to_str().context("database path must be UTF-8")?;
        let database = Builder::new_local(path).build().await?;
        let conn = connect(&database).await?;
        Ok(Self {
            database,
            writer: Arc::new(Mutex::new(Writer {
                conn,
                fk_off: false,
            })),
        })
    }

    /// A reading connection. It skips the `foreign_keys` pragma: only the writer needs it.
    // ponytail: a new connection and one pragma per call, several per page; add a small reader
    // pool if request latency or connection churn shows up under load.
    pub async fn read(&self) -> anyhow::Result<Connection> {
        let conn = self.database.connect()?;
        conn.execute("PRAGMA busy_timeout = 5000", ()).await?;
        Ok(conn)
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
    /// turso 0.8.3 answers `PRAGMA foreign_key_check` with no rows, so `f` must keep every
    /// reference valid itself.
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
        let mut w = self.writer.lock().await;
        w.clean(&self.database).await;
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
        w.clean(&self.database).await;
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

    /// Writes a consistent copy of the database to `path` (US-33). It runs on its own reading
    /// connection, so writes go on during a long copy. It refuses to overwrite: remove `path` first.
    // ponytail: turso runs the copy without yielding, so it holds one runtime worker until done;
    // move it to a dedicated thread if backups grow past a few seconds.
    pub async fn vacuum_into(&self, path: &Path) -> anyhow::Result<()> {
        let target = path
            .to_str()
            .context("backup path must be UTF-8")?
            .replace('\'', "''");
        self.read()
            .await?
            .execute(&format!("VACUUM INTO '{target}'"), ())
            .await?;
        Ok(())
    }

    /// Drops every table. Only reset-state calls this.
    pub async fn drop_all_tables(&self) -> anyhow::Result<()> {
        let names = query_all(
            &self.read().await?,
            "SELECT name FROM sqlite_schema WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
            (),
            |r| r.get::<String>(0),
        )
        .await?;
        self.write_without_fk(async |c| {
            for name in &names {
                // IF EXISTS: turso drops a table's internal AUTOINCREMENT counter with it.
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

async fn connect(database: &Database) -> anyhow::Result<Connection> {
    let conn = database.connect()?;
    conn.execute("PRAGMA busy_timeout = 5000", ()).await?;
    conn.execute("PRAGMA foreign_keys = ON", ()).await?;
    Ok(conn)
}

/// Runs a query and maps every row with `f`.
pub async fn query_all<T>(
    conn: &Connection,
    sql: impl AsRef<str>,
    params: impl turso::params::IntoParams,
    f: impl Fn(&turso::Row) -> turso::Result<T>,
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
    params: impl turso::params::IntoParams,
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
            for i in 0..20_000 {
                c.execute("INSERT INTO big VALUES (?1)", [format!("{i:0>200}")])
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
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
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
        assert_eq!(n, Some(6));
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
