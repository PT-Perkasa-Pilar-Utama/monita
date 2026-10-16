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
    /// True between BEGIN and COMMIT/ROLLBACK. A caller future dropped mid-transaction (a client
    /// that disconnects) leaves it set; the next user rolls the stale transaction back first.
    in_tx: bool,
}

impl Writer {
    async fn clean(&mut self) {
        if self.in_tx {
            tracing::warn!("rolling back a transaction left open by a dropped write");
            if let Err(e) = self.conn.execute("ROLLBACK", ()).await {
                tracing::error!(error = %e, "rollback of an abandoned transaction failed");
            }
            self.in_tx = false;
        }
    }
}

impl Db {
    pub async fn open(path: &Path) -> anyhow::Result<Self> {
        let path = path.to_str().context("database path must be UTF-8")?;
        let database = Builder::new_local(path).build().await?;
        let conn = connect(&database).await?;
        Ok(Self {
            database,
            writer: Arc::new(Mutex::new(Writer { conn, in_tx: false })),
        })
    }

    pub async fn read(&self) -> anyhow::Result<Connection> {
        connect(&self.database).await
    }

    /// Runs `f` on the writer inside `BEGIN IMMEDIATE ... COMMIT`. Rolls back when `f` fails.
    pub async fn write<T>(
        &self,
        f: impl AsyncFnOnce(&Connection) -> anyhow::Result<T>,
    ) -> anyhow::Result<T> {
        let mut w = self.writer.lock().await;
        w.clean().await;
        w.in_tx = true;
        w.conn.execute("BEGIN IMMEDIATE", ()).await?;
        let result = match f(&w.conn).await {
            Ok(v) => match w.conn.execute("COMMIT", ()).await {
                Ok(_) => {
                    w.in_tx = false;
                    return Ok(v);
                }
                Err(e) => Err(e.into()),
            },
            Err(e) => Err(e),
        };
        if let Err(rb) = w.conn.execute("ROLLBACK", ()).await {
            tracing::error!(error = %rb, "rollback failed");
        }
        w.in_tx = false;
        result
    }

    /// Writes a consistent copy of the database to `path` (US-33). `VACUUM INTO` cannot run in a
    /// transaction, so this locks the writer directly. It refuses to overwrite: remove `path` first.
    pub async fn vacuum_into(&self, path: &Path) -> anyhow::Result<()> {
        let target = path
            .to_str()
            .context("backup path must be UTF-8")?
            .replace('\'', "''");
        let mut w = self.writer.lock().await;
        w.clean().await;
        w.conn
            .execute(&format!("VACUUM INTO '{target}'"), ())
            .await?;
        Ok(())
    }

    /// Drops every table. Only reset-state calls this. Foreign keys must be off outside a
    /// transaction, so this does not use `write`.
    pub async fn drop_all_tables(&self) -> anyhow::Result<()> {
        let mut w = self.writer.lock().await;
        w.clean().await;
        let conn = &w.conn;
        let mut names = Vec::new();
        {
            let mut rows = conn
                .query(
                    "SELECT name FROM sqlite_schema WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
                    (),
                )
                .await?;
            while let Some(row) = rows.next().await? {
                names.push(row.get::<String>(0)?);
            }
        }
        conn.execute("PRAGMA foreign_keys = OFF", ()).await?;
        let result = async {
            conn.execute("BEGIN IMMEDIATE", ()).await?;
            for name in &names {
                conn.execute(&format!("DROP TABLE \"{}\"", name.replace('"', "\"\"")), ())
                    .await?;
            }
            conn.execute("COMMIT", ()).await?;
            anyhow::Ok(())
        }
        .await;
        if result.is_err() {
            let _ = conn.execute("ROLLBACK", ()).await;
        }
        conn.execute("PRAGMA foreign_keys = ON", ()).await?;
        result
    }
}

async fn connect(database: &Database) -> anyhow::Result<Connection> {
    let conn = database.connect()?;
    conn.execute("PRAGMA busy_timeout = 5000", ()).await?;
    conn.execute("PRAGMA foreign_keys = ON", ()).await?;
    Ok(conn)
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
