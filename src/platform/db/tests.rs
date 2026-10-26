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

#[tokio::test]
async fn an_old_file_switches_to_incremental_vacuum_once_and_gives_space_back() {
    let dir = tempfile::tempdir().unwrap();
    let fresh = Db::open(&dir.path().join("new.db")).await.unwrap();
    crate::platform::migrate::run(&fresh).await.unwrap();
    assert!(
        !fresh.enable_incremental_vacuum().await.unwrap(),
        "a new file has it"
    );

    let path = dir.path().join("old.db");
    let old = rusqlite::Connection::open(&path).unwrap();
    old.execute_batch(
        "CREATE TABLE t (v BLOB);
         WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 3000)
         INSERT INTO t SELECT randomblob(4000) FROM n;",
    )
    .unwrap();
    drop(old);
    let db = Db::open(&path).await.unwrap();
    db.free_pages().await.unwrap();
    assert!(db.enable_incremental_vacuum().await.unwrap());
    assert!(!db.enable_incremental_vacuum().await.unwrap(), "once");
    let count = |sql: &'static str| {
        let db = db.clone();
        async move { query_i64(&db.read().await.unwrap(), sql, ()).await.unwrap() }
    };
    assert_eq!(count("SELECT count(*) FROM t").await, Some(3000));
    let pages = count("PRAGMA page_count").await.unwrap();
    db.write(async |c| Ok(c.execute("DELETE FROM t WHERE rowid > 100", ()).await?))
        .await
        .unwrap();
    assert!(count("PRAGMA freelist_count").await.unwrap() > FREE_BATCH);
    db.free_pages().await.unwrap();
    assert_eq!(count("PRAGMA freelist_count").await, Some(0));
    assert!(count("PRAGMA page_count").await.unwrap() < pages / 10);
}
