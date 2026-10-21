//! SQL for locations and monitor_locations (technical-specs 14).

use turso::Connection;

use crate::platform::{caps, db::query_all, db::query_i64};

/// Seconds after its newest claim when a location's claim on a monitor is stale: two intervals,
/// the timeout and 30 s (technical-specs 14.6). One SQL fragment for every statement that asks,
/// so claiming and answering agree. It reads the row's monitor through `monitor_locations`.
pub const STALE_AFTER: &str = "(SELECT 2 * interval_s + timeout_ms / 1000 + 30 FROM monitors WHERE id = monitor_locations.monitor_id)";

#[derive(serde::Serialize)]
pub struct LocationRow {
    pub id: i64,
    pub name: String,
    pub token_prefix: String,
    pub last_seen_at: Option<i64>,
    /// How many monitors also check from it.
    pub monitors: i64,
    pub created_at: i64,
}

pub async fn list(c: &Connection) -> anyhow::Result<Vec<LocationRow>> {
    query_all(
        c,
        "SELECT l.id, l.name, l.token_prefix, l.last_seen_at,
           (SELECT count(*) FROM monitor_locations WHERE location_id = l.id), l.created_at
         FROM locations l ORDER BY l.name LIMIT ?1",
        [caps::LOCATIONS],
        |r| {
            Ok(LocationRow {
                id: r.get(0)?,
                name: r.get(1)?,
                token_prefix: r.get(2)?,
                last_seen_at: r.get(3)?,
                monitors: r.get(4)?,
                created_at: r.get(5)?,
            })
        },
    )
    .await
}

pub async fn count(c: &Connection) -> anyhow::Result<i64> {
    Ok(query_i64(c, "SELECT count(*) FROM locations", ())
        .await?
        .unwrap_or(0))
}

pub async fn name_taken(c: &Connection, name: &str) -> anyhow::Result<bool> {
    Ok(
        query_i64(c, "SELECT count(*) FROM locations WHERE name = ?1", [name])
            .await?
            .unwrap_or(0)
            > 0,
    )
}

pub async fn insert(
    c: &Connection,
    name: &str,
    token_hash: &str,
    token_prefix: &str,
    at: i64,
) -> anyhow::Result<i64> {
    c.execute(
        "INSERT INTO locations (name, token_hash, token_prefix, created_at) VALUES (?1, ?2, ?3, ?4)",
        (name, token_hash, token_prefix, at),
    )
    .await?;
    Ok(c.last_insert_rowid())
}

/// Deletes the location; its assignments and results cascade. Each monitor's quorum then drops to
/// at most its number of places. False when there was no such location.
pub async fn delete(c: &Connection, id: i64) -> anyhow::Result<bool> {
    let gone = c
        .execute("DELETE FROM locations WHERE id = ?1", [id])
        .await?
        > 0;
    if gone {
        c.execute(
            "UPDATE monitors SET quorum = 1 + (SELECT count(*) FROM monitor_locations WHERE monitor_id = monitors.id)
             WHERE quorum > 1 + (SELECT count(*) FROM monitor_locations WHERE monitor_id = monitors.id)",
            (),
        )
        .await?;
    }
    Ok(gone)
}

/// The location a token's hash belongs to: id and when it last called.
pub async fn by_token(c: &Connection, hash: &str) -> anyhow::Result<Option<(i64, Option<i64>)>> {
    Ok(query_all(
        c,
        "SELECT id, last_seen_at FROM locations WHERE token_hash = ?1",
        [hash],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .await?
    .pop())
}

pub async fn touch(c: &Connection, id: i64, at: i64) -> anyhow::Result<()> {
    c.execute(
        "UPDATE locations SET last_seen_at = ?1 WHERE id = ?2",
        (at, id),
    )
    .await?;
    Ok(())
}

/// A read-only probe, so an idle poll never waits for the writer.
pub async fn any_due(c: &Connection, location_id: i64, at: i64) -> anyhow::Result<bool> {
    Ok(query_i64(
        c,
        "SELECT EXISTS (SELECT 1 FROM monitor_locations ml JOIN monitors m ON m.id = ml.monitor_id
         WHERE ml.location_id = ?1 AND ml.next_check_at <= ?2 AND m.paused_at IS NULL)",
        (location_id, at),
    )
    .await?
        == Some(1))
}

/// Every location, as `(id, name)`, for the monitor form.
pub async fn names(c: &Connection) -> anyhow::Result<Vec<(i64, String)>> {
    query_all(
        c,
        "SELECT id, name FROM locations ORDER BY name LIMIT ?1",
        [caps::LOCATIONS],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .await
}

/// The locations a monitor also checks from, as `(id, name)`, by name.
pub async fn of_monitor(c: &Connection, monitor_id: i64) -> anyhow::Result<Vec<(i64, String)>> {
    query_all(
        c,
        "SELECT l.id, l.name FROM monitor_locations ml JOIN locations l ON l.id = ml.location_id
         WHERE ml.monitor_id = ?1 ORDER BY l.name LIMIT ?2",
        (monitor_id, caps::LOCATIONS),
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .await
}

/// Spends one of the location's open claims on the monitor. False when there is none to spend:
/// the location does not check it, every claim was answered, or the newest claim is older than
/// two intervals, the timeout and 30 seconds, so a result now would be stale.
pub async fn answer_claim(
    c: &Connection,
    monitor_id: i64,
    location_id: i64,
    now: i64,
) -> anyhow::Result<bool> {
    Ok(c.execute(
        &format!(
            "UPDATE monitor_locations SET open_claims = open_claims - 1
         WHERE monitor_id = ?1 AND location_id = ?2 AND open_claims > 0
           AND claimed_at >= ?3 - {STALE_AFTER}"
        ),
        (monitor_id, location_id, now),
    )
    .await?
        > 0)
}

/// Makes the monitor's locations exactly `ids`. A kept location keeps its schedule; a new one
/// first checks at `first`.
pub async fn set_for_monitor(
    c: &Connection,
    monitor_id: i64,
    ids: &[i64],
    first: i64,
) -> anyhow::Result<()> {
    for (id, _) in of_monitor(c, monitor_id).await? {
        if !ids.contains(&id) {
            c.execute(
                "DELETE FROM monitor_locations WHERE monitor_id = ?1 AND location_id = ?2",
                (monitor_id, id),
            )
            .await?;
        }
    }
    for id in ids {
        c.execute(
            "INSERT OR IGNORE INTO monitor_locations (monitor_id, location_id, next_check_at) VALUES (?1, ?2, ?3)",
            (monitor_id, *id, first),
        )
        .await?;
    }
    Ok(())
}

/// The newest result, by time, of each place: `(location_id, checked_at, ok, error)`, None for
/// this server. One indexed lookup per place, so at most `LOCATIONS + 1`; a place with no result
/// is left out.
pub async fn newest_by_place(
    c: &Connection,
    monitor_id: i64,
    places: &[Option<i64>],
) -> anyhow::Result<Vec<(Option<i64>, i64, bool, Option<String>)>> {
    let mut out = Vec::new();
    for &place in places {
        let row = query_all(
            c,
            "SELECT checked_at, ok, error FROM check_results WHERE monitor_id = ?1 AND location_id IS ?2
             ORDER BY checked_at DESC, id DESC LIMIT 1",
            (monitor_id, place),
            |r| Ok((place, r.get(0)?, r.get::<i64>(1)? == 1, r.get(2)?)),
        )
        .await?;
        out.extend(row);
    }
    Ok(out)
}
