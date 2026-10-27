//! Another process may read the live database, as `sqlite3` does when an admin looks at it. On
//! Unix, closing any handle to a file drops every lock the process holds on it. If Monita opened
//! and closed the database file after SQLite had it open, the other process would believe it was
//! alone, delete the WAL when it closed, and leave Monita with "disk I/O error" until a restart.

use monita::platform::db::{Db, query_i64};

/// The other process: opens the file with SQLite, reads, and closes, as `sqlite3` would.
#[test]
fn other_process_reads_the_file() {
    let Ok(path) = std::env::var("MONITA_SHARED_DB") else {
        return;
    };
    let c = rusqlite::Connection::open(&path).unwrap();
    let n: i64 = c
        .query_row("SELECT count(*) FROM notes", [], |r| r.get(0))
        .unwrap();
    assert!(n > 0);
}

#[tokio::test]
async fn another_process_reading_the_live_file_leaves_monita_working() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("m.db");
    // A file an earlier run left, closed cleanly, as on an upgrade or a restart.
    {
        let c = rusqlite::Connection::open(&path).unwrap();
        c.execute_batch(
            "PRAGMA journal_mode = WAL; CREATE TABLE notes (n INTEGER); INSERT INTO notes VALUES (1);",
        )
        .unwrap();
    }
    let db = Db::open(&path).await.unwrap();
    db.write(async |c| {
        c.execute("INSERT INTO notes VALUES (2)", ()).await?;
        Ok(())
    })
    .await
    .unwrap();
    let wal = dir.path().join("m.db-wal");
    assert!(wal.exists(), "Monita works in WAL mode");

    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["other_process_reads_the_file", "--exact", "--quiet"])
        .env("MONITA_SHARED_DB", &path)
        .output()
        .unwrap();
    assert!(child.status.success(), "{child:?}");

    assert!(
        wal.exists(),
        "the other process saw Monita's locks and kept the WAL"
    );
    db.write(async |c| {
        c.execute("INSERT INTO notes VALUES (3)", ()).await?;
        Ok(())
    })
    .await
    .unwrap();
    let n = query_i64(&db.read().await.unwrap(), "SELECT count(*) FROM notes", ())
        .await
        .unwrap();
    assert_eq!(n, Some(3));
}
