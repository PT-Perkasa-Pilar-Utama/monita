//! Time display. Storage is always Unix seconds, UTC.
// ponytail: UTC only; add a display offset setting (WIB) when TLs ask for local time.

/// `2026-10-16 08:00 UTC`
pub fn utc(ts: i64) -> String {
    let days = ts.div_euclid(86_400);
    let secs = ts.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02} UTC",
        secs / 3600,
        (secs % 3600) / 60
    )
}

/// Unix seconds of the UTC midnight that starts the day of `ts`.
pub fn day_start(ts: i64) -> i64 {
    ts - ts.rem_euclid(86_400)
}

/// `2026-10-16`, the UTC day a timestamp falls on.
pub fn day(ts: i64) -> String {
    let (y, m, d) = civil_from_days(ts.div_euclid(86_400));
    format!("{y:04}-{m:02}-{d:02}")
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
    fn formats_utc_times() {
        assert_eq!(utc(0), "1970-01-01 00:00 UTC");
        assert_eq!(utc(1_792_161_874), "2026-10-16 14:44 UTC");
        assert_eq!(day(1_792_137_600), "2026-10-16");
    }

    #[test]
    fn formats_durations_and_ages() {
        assert_eq!(duration(378), "6m 18s");
        assert_eq!(duration(7_440), "2h 4m");
        assert_eq!(ago(100, 100 + 360), "6m ago");
        assert_eq!(ago(100, 110), "just now");
    }
}
