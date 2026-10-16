//! Daily rollup into daily_summaries, then the retention sweep (US-32, spec 13 section 13.2).

use crate::{
    incidents::{self, SourceKind},
    platform::{db::Db, settings},
};

use super::store::rollup as store;

const RAW_KEEP_S: i64 = 7 * 86_400;
const OUTBOX_KEEP_S: i64 = 30 * 86_400;
const BACKFILL_DAYS: i64 = 90;
/// Unix start of the last summarized day. Not `last_rollup_day`: the job runner owns that key.
const LAST_DAY: &str = "rollup_through";

/// Summarizes every day from the last rollup to yesterday (90 days at most), then deletes raw
/// check results and pings older than 7 days, delivered outbox rows older than 30 days, and
/// expired sessions and invites. Roll up first, so the sweep never deletes unsummarized rows.
pub async fn run(db: &Db, now: i64) -> anyhow::Result<()> {
    let today = crate::platform::time::day_start(now);
    let yesterday = today - 86_400;
    let last = settings::get_raw(&db.read().await?, LAST_DAY)
        .await?
        .and_then(|v| v.parse::<i64>().ok());
    let first = last
        .map_or(yesterday, |l| l + 86_400)
        .max(yesterday - (BACKFILL_DAYS - 1) * 86_400);
    let mut day = first;
    while day <= yesterday {
        rollup_day(db, day, now).await?;
        let d = day.to_string();
        db.write(async |c| settings::set_raw(c, LAST_DAY, &d).await)
            .await?;
        day += 86_400;
    }
    let raw_cutoff = now - RAW_KEEP_S;
    let outbox_cutoff = now - OUTBOX_KEEP_S;
    db.write(async |c| store::sweep(c, raw_cutoff, outbox_cutoff, now).await)
        .await
}

async fn rollup_day(db: &Db, day: i64, now: i64) -> anyhow::Result<()> {
    let conn = db.read().await?;
    let mut rows = Vec::new();
    for (kind, id) in store::all_sources(&conn).await? {
        let (total, ok, latency) = match kind {
            SourceKind::Monitor => store::check_counts(&conn, id, day).await?,
            SourceKind::Heartbeat => store::ping_counts(&conn, id, day).await?,
        };
        let spans = incidents::history::source_intervals(db, kind, id, day, now).await?;
        let down = incidents::history::per_day(&spans, day, 1)[0];
        rows.push((kind, id, total, ok, down, latency));
    }
    drop(conn);
    let label = crate::platform::time::day(day);
    db.write(async |c| {
        for (kind, id, total, ok, down, latency) in &rows {
            store::upsert_summary(c, *kind, *id, &label, *total, *ok, *down, *latency).await?;
        }
        Ok(())
    })
    .await
}
