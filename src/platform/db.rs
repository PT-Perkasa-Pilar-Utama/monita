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
    writer: Arc<Mutex<Connection>>,
}

impl Db {
    pub async fn open(path: &Path) -> anyhow::Result<Self> {
        let path = path.to_str().context("database path must be UTF-8")?;
        let database = Builder::new_local(path).build().await?;
        let writer = connect(&database).await?;
        Ok(Self {
            database,
            writer: Arc::new(Mutex::new(writer)),
        })
    }

    /// A fresh connection for reads.
    pub async fn read(&self) -> anyhow::Result<Connection> {
        connect(&self.database).await
    }

    /// Runs `f` on the writer inside `BEGIN IMMEDIATE ... COMMIT`. Rolls back when `f` fails.
    pub async fn write<T>(
        &self,
        f: impl AsyncFnOnce(&Connection) -> anyhow::Result<T>,
    ) -> anyhow::Result<T> {
        let conn = self.writer.lock().await;
        conn.execute("BEGIN IMMEDIATE", ()).await?;
        match f(&conn).await {
            Ok(v) => {
                conn.execute("COMMIT", ()).await?;
                Ok(v)
            }
            Err(e) => {
                if let Err(rb) = conn.execute("ROLLBACK", ()).await {
                    tracing::error!(error = %rb, "rollback failed");
                }
                Err(e)
            }
        }
    }

    /// Writes a consistent copy of the database to `path` (US-33). `VACUUM INTO` cannot run in a
    /// transaction, so this locks the writer directly. It refuses to overwrite: remove `path` first.
    pub async fn vacuum_into(&self, path: &Path) -> anyhow::Result<()> {
        let target = path
            .to_str()
            .context("backup path must be UTF-8")?
            .replace('\'', "''");
        let conn = self.writer.lock().await;
        conn.execute(&format!("VACUUM INTO '{target}'"), ()).await?;
        Ok(())
    }

    /// Drops every table. Only reset-state calls this. Foreign keys must be off outside a
    /// transaction, so this does not use `write`.
    pub async fn drop_all_tables(&self) -> anyhow::Result<()> {
        let conn = self.writer.lock().await;
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
}
