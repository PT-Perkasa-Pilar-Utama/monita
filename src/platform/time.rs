//! Time display. Storage is always Unix seconds, UTC. Display shifts by the fixed UTC offset a
//! super admin sets (`settings::utc_offset`), in minutes east of UTC.
// ponytail: a fixed offset, no daylight saving; switch to named zones (a tz database) if a team
// in a DST country needs it. Stored daily summaries stay UTC days.

/// `UTC`, `UTC+7`, `UTC+5:30`, `UTC-3`.
pub fn zone(off: i64) -> String {
    if off == 0 {
        return "UTC".into();
    }
    let sign = if off > 0 { '+' } else { '-' };
    let (h, m) = (off.abs() / 60, off.abs() % 60);
    if m == 0 {
        format!("UTC{sign}{h}")
    } else {
        format!("UTC{sign}{h}:{m:02}")
    }
}

/// `2026-10-16 15:00 UTC+7`: the time in the offset, with the offset named.
pub fn stamp(ts: i64, off: i64) -> String {
    format!("{} {}", local(ts, off), zone(off))
}

/// `2026-10-16 15:00`, the time in the offset without its name.
pub fn local(ts: i64, off: i64) -> String {
    let t = ts + off * 60;
    let secs = t.rem_euclid(86_400);
    format!("{} {:02}:{:02}", day(t), secs / 3600, (secs % 3600) / 60)
}

/// Unix seconds of the UTC midnight that starts the day of `ts`.
pub fn day_start(ts: i64) -> i64 {
    ts - ts.rem_euclid(86_400)
}

/// Unix seconds of the midnight, in the offset, that starts the day of `ts` there.
pub fn day_start_at(ts: i64, off: i64) -> i64 {
    day_start(ts + off * 60) - off * 60
}

/// `2026-10-16`, the UTC day a timestamp falls on.
pub fn day(ts: i64) -> String {
    let (y, m, d) = civil_from_days(ts.div_euclid(86_400));
    format!("{y:04}-{m:02}-{d:02}")
}

/// The day a timestamp falls on in the offset.
pub fn day_at(ts: i64, off: i64) -> String {
    day(ts + off * 60)
}

/// Unix seconds of the midnight, in the offset, that starts the month of `ts` there.
pub fn month_start_at(ts: i64, off: i64) -> i64 {
    let local_day = (ts + off * 60).div_euclid(86_400);
    let (_, _, d) = civil_from_days(local_day);
    (local_day - i64::from(d) + 1) * 86_400 - off * 60
}

/// The start of the month after the one `start` begins.
pub fn next_month_at(start: i64, off: i64) -> i64 {
    month_start_at(start + 32 * 86_400, off)
}

/// The start of the month before the one `start` begins.
pub fn prev_month_at(start: i64, off: i64) -> i64 {
    month_start_at(start - 86_400, off)
}

/// `2026-10`, the month a timestamp falls in, in the offset.
pub fn month_at(ts: i64, off: i64) -> String {
    day_at(ts, off)[..7].to_string()
}

/// `45s`, `6m 18s`, `2h 4m`, `3d 2h`.
pub fn duration(secs: i64) -> String {
    let s = secs.max(0);
    match s {
        0..60 => format!("{s}s"),
        60..3600 => format!("{}m {}s", s / 60, s % 60),
        3600..86_400 => format!("{}h {}m", s / 3600, (s % 3600) / 60),
        _ => format!("{}d {}h", s / 86_400, (s % 86_400) / 3600),
    }
}

/// `just now`, `6m ago`, `2h ago`, `3d ago`.
pub fn ago(ts: i64, now: i64) -> String {
    let s = (now - ts).max(0);
    match s {
        0..60 => "just now".into(),
        60..3600 => format!("{}m ago", s / 60),
        3600..86_400 => format!("{}h ago", s / 3600),
        _ => format!("{}d ago", s / 86_400),
    }
}

/// Days since 1970-01-01 for a calendar date: Howard Hinnant's civil-to-days algorithm, the
/// inverse of `civil_from_days`.
pub fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = y - i64::from(m <= 2);
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let m = i64::from(m);
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Howard Hinnant's days-to-civil algorithm.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_dates_and_day_numbers_round_trip() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(2000, 3, 1), 11_017);
        for z in [-1, 0, 59, 60, 11_016, 20_746, 30_000, 100_000] {
            let (y, m, d) = civil_from_days(z);
            assert_eq!(days_from_civil(y, m, d), z, "{y}-{m}-{d}");
        }
    }

    #[test]
    fn formats_times_in_the_offset() {
        assert_eq!(stamp(0, 0), "1970-01-01 00:00 UTC");
        assert_eq!(stamp(1_792_161_874, 0), "2026-10-16 14:44 UTC");
        assert_eq!(stamp(1_792_161_874, 420), "2026-10-16 21:44 UTC+7");
        assert_eq!(stamp(1_792_161_874, 330), "2026-10-16 20:14 UTC+5:30");
        assert_eq!(stamp(1_792_108_800 + 3600, -180), "2026-10-15 22:00 UTC-3");
        assert_eq!(day(1_792_137_600), "2026-10-16");
    }

    #[test]
    fn a_day_starts_at_midnight_in_the_offset() {
        // 2026-10-16 02:00 UTC is 09:00 at UTC+7; that day began at 2026-10-15 17:00 UTC.
        let t = 1_792_108_800 + 2 * 3600;
        assert_eq!(day_start_at(t, 420), 1_792_108_800 - 7 * 3600);
        assert_eq!(day_start_at(t, 0), 1_792_108_800);
        // 2026-10-16 02:00 UTC is still 2026-10-15 at UTC-3.
        assert_eq!(day_at(t, -180), "2026-10-15");
    }

    #[test]
    fn a_month_starts_at_its_first_midnight_in_the_offset() {
        const OCT_1: i64 = 1_790_812_800;
        const NOV_1: i64 = 1_793_491_200;
        let mid_oct = 1_792_108_800 + 2 * 3600;
        assert_eq!(month_start_at(mid_oct, 0), OCT_1);
        assert_eq!(month_start_at(mid_oct, 420), OCT_1 - 7 * 3600);
        // 2026-10-31 20:00 UTC is already November at UTC+7, still October at UTC.
        let late = NOV_1 - 4 * 3600;
        assert_eq!(month_start_at(late, 420), NOV_1 - 7 * 3600);
        assert_eq!(month_start_at(late, 0), OCT_1);
        assert_eq!(next_month_at(OCT_1, 0), NOV_1);
        assert_eq!(prev_month_at(NOV_1, 0), OCT_1);
        assert_eq!(month_at(late, 420), "2026-11");
    }

    #[test]
    fn formats_durations_and_ages() {
        assert_eq!(duration(378), "6m 18s");
        assert_eq!(duration(7_440), "2h 4m");
        assert_eq!(ago(100, 100 + 360), "6m ago");
        assert_eq!(ago(100, 110), "just now");
    }
}
