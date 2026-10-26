//! The SQLite connection every store uses: rusqlite underneath, called the way the stores read,
//! `c.query(sql, params).await?` and `rows.next().await?`. Each call runs to its end on the
//! calling thread, as a file read does, and holds the connection's lock only for that call, so a
//! `&Connection` can live across an `.await` in a handler. `query` collects its rows before it
//! returns: every multi-row query has a `LIMIT` (CLAUDE.md, no unbounded list).

use std::collections::VecDeque;

pub use rusqlite::Error;
pub use rusqlite::types::Value;
use rusqlite::types::{FromSql, FromSqlError, ToSql, ToSqlOutput, ValueRef};

pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Statements a connection keeps prepared. A page runs a few dozen distinct ones.
const CACHED_STATEMENTS: usize = 128;

/// Idle connections that go back to their pool when dropped (`Db::read`).
#[derive(Default)]
pub(crate) struct Pool {
    idle: std::sync::Mutex<Vec<rusqlite::Connection>>,
    max_idle: usize,
}

impl Pool {
    pub(crate) fn new(max_idle: usize) -> Self {
        Self {
            idle: Default::default(),
            max_idle,
        }
    }

    pub(crate) fn take(&self) -> Option<rusqlite::Connection> {
        self.idle.lock().unwrap_or_else(|e| e.into_inner()).pop()
    }
}

pub struct Connection {
    /// `None` only while `Drop` hands the connection back to its pool.
    conn: std::sync::Mutex<Option<rusqlite::Connection>>,
    pool: Option<std::sync::Arc<Pool>>,
}

struct Guard<'a>(std::sync::MutexGuard<'a, Option<rusqlite::Connection>>);

impl std::ops::Deref for Guard<'_> {
    type Target = rusqlite::Connection;
    fn deref(&self) -> &rusqlite::Connection {
        self.0
            .as_ref()
            .expect("a connection is present until it is dropped")
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        let Some(pool) = &self.pool else { return };
        let conn = self
            .conn
            .get_mut()
            .unwrap_or_else(|e| e.into_inner())
            .take();
        // A pooled connection never opens a transaction; one that somehow did is closed instead.
        let Some(conn) = conn.filter(|c| c.is_autocommit()) else {
            return;
        };
        let mut idle = pool.idle.lock().unwrap_or_else(|e| e.into_inner());
        if idle.len() < pool.max_idle {
            idle.push(conn);
        }
    }
}

impl Connection {
    pub(crate) fn new(conn: rusqlite::Connection) -> Self {
        Self::make(conn, None)
    }

    /// A connection that returns to `pool` when dropped.
    pub(crate) fn pooled(conn: rusqlite::Connection, pool: std::sync::Arc<Pool>) -> Self {
        Self::make(conn, Some(pool))
    }

    fn make(conn: rusqlite::Connection, pool: Option<std::sync::Arc<Pool>>) -> Self {
        conn.set_prepared_statement_cache_capacity(CACHED_STATEMENTS);
        Self {
            conn: std::sync::Mutex::new(Some(conn)),
            pool,
        }
    }

    fn lock(&self) -> Guard<'_> {
        // A panic while the lock was held leaves the connection as SQLite left it; SQLite has
        // no state a half-run Rust call could break, so the lock is taken back.
        Guard(self.conn.lock().unwrap_or_else(|e| e.into_inner()))
    }

    /// Runs a statement that returns rows and collects them.
    pub async fn query(&self, sql: impl AsRef<str>, params: impl IntoParams) -> Result<Rows> {
        let values = params.into_values()?;
        let conn = self.lock();
        let mut stmt = conn.prepare_cached(sql.as_ref())?;
        let width = stmt.column_count();
        let mut rows = stmt.query(rusqlite::params_from_iter(values))?;
        let mut out = VecDeque::new();
        while let Some(r) = rows.next()? {
            let row = (0..width)
                .map(|i| r.get::<_, Value>(i))
                .collect::<Result<Vec<_>>>()?;
            out.push_back(Row(row));
        }
        Ok(Rows(out))
    }

    /// Runs a statement that returns no rows; answers how many rows it changed.
    pub async fn execute(&self, sql: impl AsRef<str>, params: impl IntoParams) -> Result<u64> {
        let values = params.into_values()?;
        let conn = self.lock();
        let mut stmt = conn.prepare_cached(sql.as_ref())?;
        Ok(stmt.execute(rusqlite::params_from_iter(values))? as u64)
    }

    /// Runs several statements, such as a migration file.
    pub async fn execute_batch(&self, sql: impl AsRef<str>) -> Result<()> {
        self.lock().execute_batch(sql.as_ref())
    }

    pub fn last_insert_rowid(&self) -> i64 {
        self.lock().last_insert_rowid()
    }

    pub fn is_autocommit(&self) -> Result<bool> {
        Ok(self.lock().is_autocommit())
    }

    /// The rusqlite connection itself, for what the call shape above does not cover: pragmas
    /// that answer a row, and `VACUUM INTO`.
    pub(crate) fn with<T>(&self, f: impl FnOnce(&rusqlite::Connection) -> T) -> T {
        f(&self.lock())
    }
}

/// The rows of one `query`.
pub struct Rows(VecDeque<Row>);

impl Rows {
    pub async fn next(&mut self) -> Result<Option<Row>> {
        Ok(self.0.pop_front())
    }
}

/// One row, its values owned.
pub struct Row(Vec<Value>);

impl Row {
    /// The value at `idx`, converted the way rusqlite converts a column: an integer column reads
    /// as `i64` or `bool`, NULL only as an `Option`.
    pub fn get<T: FromSql>(&self, idx: usize) -> Result<T> {
        let value = self.0.get(idx).ok_or(Error::InvalidColumnIndex(idx))?;
        let raw = ValueRef::from(value);
        T::column_result(raw).map_err(|e| match e {
            FromSqlError::InvalidType => {
                Error::InvalidColumnType(idx, idx.to_string(), raw.data_type())
            }
            other => Error::FromSqlConversionFailure(idx, raw.data_type(), Box::new(other)),
        })
    }
}

/// Positional parameters: `()`, a tuple of up to 16 values, an array or a `Vec` of one type.
pub trait IntoParams {
    fn into_values(self) -> Result<Vec<Value>>;
}

fn owned(v: &dyn ToSql) -> Result<Value> {
    match v.to_sql()? {
        ToSqlOutput::Borrowed(r) => Ok(match r {
            ValueRef::Null => Value::Null,
            ValueRef::Integer(i) => Value::Integer(i),
            ValueRef::Real(f) => Value::Real(f),
            ValueRef::Text(t) => Value::Text(String::from_utf8_lossy(t).into_owned()),
            ValueRef::Blob(b) => Value::Blob(b.to_vec()),
        }),
        ToSqlOutput::Owned(v) => Ok(v),
        _ => Err(Error::ToSqlConversionFailure(
            "unsupported parameter kind".into(),
        )),
    }
}

impl IntoParams for () {
    fn into_values(self) -> Result<Vec<Value>> {
        Ok(Vec::new())
    }
}

impl<T: ToSql, const N: usize> IntoParams for [T; N] {
    fn into_values(self) -> Result<Vec<Value>> {
        self.iter().map(|v| owned(v)).collect()
    }
}

impl<T: ToSql> IntoParams for Vec<T> {
    fn into_values(self) -> Result<Vec<Value>> {
        self.iter().map(|v| owned(v)).collect()
    }
}

macro_rules! tuple_params {
    ($($t:ident $i:tt),+) => {
        impl<$($t: ToSql),+> IntoParams for ($($t,)+) {
            fn into_values(self) -> Result<Vec<Value>> {
                Ok(vec![$(owned(&self.$i)?),+])
            }
        }
    };
}

tuple_params!(A 0);
tuple_params!(A 0, B 1);
tuple_params!(A 0, B 1, C 2);
tuple_params!(A 0, B 1, C 2, D 3);
tuple_params!(A 0, B 1, C 2, D 3, E 4);
tuple_params!(A 0, B 1, C 2, D 3, E 4, F 5);
tuple_params!(A 0, B 1, C 2, D 3, E 4, F 5, G 6);
tuple_params!(A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7);
tuple_params!(A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8);
tuple_params!(A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8, J 9);
tuple_params!(A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8, J 9, K 10);
tuple_params!(A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8, J 9, K 10, L 11);
tuple_params!(A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8, J 9, K 10, L 11, M 12);
tuple_params!(A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8, J 9, K 10, L 11, M 12, N 13);
tuple_params!(A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8, J 9, K 10, L 11, M 12, N 13, O 14);
tuple_params!(A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8, J 9, K 10, L 11, M 12, N 13, O 14, P 15);

#[cfg(test)]
mod tests {
    use super::*;

    fn conn() -> Connection {
        Connection::new(rusqlite::Connection::open_in_memory().unwrap())
    }

    #[tokio::test]
    async fn params_rows_and_types_round_trip() {
        let c = conn();
        c.execute_batch("CREATE TABLE t (i INTEGER, s TEXT, b BLOB, f REAL, n INTEGER)")
            .await
            .unwrap();
        let blob = vec![0_u8, 1, 255];
        let n = c
            .execute(
                "INSERT INTO t VALUES (?1, ?2, ?3, ?4, ?5)",
                (7_i64, "seven", blob.clone(), 0.5_f64, None::<i64>),
            )
            .await
            .unwrap();
        assert_eq!(n, 1);
        assert_eq!(c.last_insert_rowid(), 1);
        c.execute("INSERT INTO t (i, s) VALUES (?1, ?2)", [8_i64, 9])
            .await
            .unwrap();
        c.execute("INSERT INTO t (s) VALUES (?1)", vec!["v".to_string()])
            .await
            .unwrap();
        c.execute("INSERT INTO t (i) VALUES (?1)", (true,))
            .await
            .unwrap();
        let mut rows = c
            .query("SELECT i, s, b, f, n FROM t ORDER BY rowid", ())
            .await
            .unwrap();
        let first = rows.next().await.unwrap().unwrap();
        assert_eq!(first.get::<i64>(0).unwrap(), 7);
        assert_eq!(first.get::<String>(1).unwrap(), "seven");
        assert_eq!(first.get::<Vec<u8>>(2).unwrap(), blob);
        assert_eq!(first.get::<f64>(3).unwrap(), 0.5);
        assert_eq!(first.get::<Option<i64>>(4).unwrap(), None);
        assert!(first.get::<i64>(4).is_err(), "NULL reads only as an Option");
        assert!(first.get::<i64>(9).is_err(), "a missing column is an error");
        let second = rows.next().await.unwrap().unwrap();
        assert_eq!(
            second.get::<String>(1).unwrap(),
            "9",
            "a TEXT column stores a number as text"
        );
        rows.next().await.unwrap().unwrap();
        assert!(rows.next().await.unwrap().unwrap().get::<bool>(0).unwrap());
        assert!(rows.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn a_select_through_execute_is_an_error() {
        let c = conn();
        assert!(c.execute("SELECT 1", ()).await.is_err());
        assert!(c.is_autocommit().unwrap());
        c.execute_batch("BEGIN").await.unwrap();
        assert!(!c.is_autocommit().unwrap());
    }
}
