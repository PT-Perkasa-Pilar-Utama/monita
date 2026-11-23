//! Which days a report covers (v1.2 Q59), in the admin's offset. Every range is whole days,
//! `[from, to)`, and ends at or before now.

use crate::platform::time;

/// The longest range a report covers.
pub const MAX_DAYS: i64 = 366;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Range {
    pub from: i64,
    pub to: i64,
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum RangeError {
    #[error("Pick a first and a last day as YYYY-MM-DD.")]
    BadDay,
    #[error("The last day comes before the first.")]
    Backwards,
    #[error("The first day is in the future.")]
    Future,
    #[error("A report covers {MAX_DAYS} days at most.")]
    TooLong,
}

/// The seven whole days before today.
pub fn last_7_days(now: i64, off: i64) -> Range {
    let today = time::day_start_at(now, off);
    Range {
        from: today - 7 * 86_400,
        to: today,
    }
}

/// The calendar month before this one.
pub fn last_month(now: i64, off: i64) -> Range {
    let this = time::month_start_at(now, off);
    Range {
        from: time::prev_month_at(this, off),
        to: this,
    }
}

/// The calendar quarter (January to March, April to June, and so on) before this one.
pub fn last_quarter(now: i64, off: i64) -> Range {
    let mut start = time::month_start_at(now, off);
    // Back to the first month of this quarter: January, April, July or October.
    while month_of(start, off) % 3 != 1 {
        start = time::prev_month_at(start, off);
    }
    let mut from = start;
    for _ in 0..3 {
        from = time::prev_month_at(from, off);
    }
    Range { from, to: start }
}

/// Monday to Sunday of the week before this one (v1.2 Q59).
pub fn last_week(now: i64, off: i64) -> Range {
    let today = time::day_start_at(now, off);
    // 1970-01-01 was a Thursday: days since then, plus 3, count from a Monday.
    let local_day = (today + off * 60).div_euclid(86_400);
    let monday = today - (local_day + 3).rem_euclid(7) * 86_400;
    Range {
        from: monday - 7 * 86_400,
        to: monday,
    }
}

fn month_of(ts: i64, off: i64) -> u32 {
    time::day_at(ts, off)[5..7].parse().unwrap_or(1)
}

/// The days from `from` to `to`, both `YYYY-MM-DD` and both included. A last day that has not
/// ended yet ends now.
pub fn days(from: &str, to: &str, now: i64, off: i64) -> Result<Range, RangeError> {
    let (Some(start), Some(end)) = (time::parse_day_at(from, off), time::parse_day_at(to, off))
    else {
        return Err(RangeError::BadDay);
    };
    let end = end + 86_400;
    if end <= start {
        return Err(RangeError::Backwards);
    }
    if start >= now {
        return Err(RangeError::Future);
    }
    if (end - start) / 86_400 > MAX_DAYS {
        return Err(RangeError::TooLong);
    }
    Ok(Range {
        from: start,
        to: end.min(now),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // 2026-11-23 10:00 UTC, a Monday; 17:00 in UTC+7.
    const NOW: i64 = 1_795_428_000;
    const WIB: i64 = 420;

    fn days_of(r: Range, off: i64) -> (String, String) {
        (time::day_at(r.from, off), time::day_at(r.to - 1, off))
    }

    #[test]
    fn the_fixed_ranges_are_whole_days_that_end_before_today() {
        assert_eq!(time::day_at(NOW, WIB), "2026-11-23");
        assert_eq!(
            days_of(last_7_days(NOW, WIB), WIB),
            ("2026-11-16".into(), "2026-11-22".into())
        );
        assert_eq!(
            days_of(last_month(NOW, WIB), WIB),
            ("2026-10-01".into(), "2026-10-31".into())
        );
        assert_eq!(
            days_of(last_quarter(NOW, WIB), WIB),
            ("2026-07-01".into(), "2026-09-30".into())
        );
        assert_eq!(
            days_of(last_week(NOW, WIB), WIB),
            ("2026-11-16".into(), "2026-11-22".into())
        );
        assert_eq!(
            last_month(NOW, WIB).from,
            time::parse_day_at("2026-10-01", WIB).unwrap()
        );
    }

    #[test]
    fn a_week_starts_on_monday_whatever_day_today_is() {
        for d in 0..7 {
            let r = last_week(NOW + d * 86_400, WIB);
            assert_eq!(
                days_of(r, WIB),
                ("2026-11-16".into(), "2026-11-22".into()),
                "day {d}"
            );
        }
        assert_eq!(
            days_of(last_week(NOW + 7 * 86_400, WIB), WIB).0,
            "2026-11-23"
        );
    }

    #[test]
    fn the_first_quarter_reaches_back_into_last_year() {
        let feb = time::parse_day_at("2027-02-10", 0).unwrap();
        assert_eq!(
            days_of(last_quarter(feb, 0), 0),
            ("2026-10-01".into(), "2026-12-31".into())
        );
        let jan = time::parse_day_at("2027-01-01", 0).unwrap();
        assert_eq!(
            days_of(last_month(jan, 0), 0),
            ("2026-12-01".into(), "2026-12-31".into())
        );
    }

    #[test]
    fn a_range_of_days_is_checked_and_ends_now_at_the_latest() {
        let r = days("2026-11-01", "2026-11-30", NOW, WIB).unwrap();
        assert_eq!(r.from, time::parse_day_at("2026-11-01", WIB).unwrap());
        assert_eq!(r.to, NOW, "today has not ended");
        let one = days("2026-11-02", "2026-11-02", NOW, WIB).unwrap();
        assert_eq!(one.to - one.from, 86_400);
        assert_eq!(
            days("2026-11-31", "2026-12-01", NOW, WIB),
            Err(RangeError::BadDay)
        );
        assert_eq!(days("", "2026-11-01", NOW, WIB), Err(RangeError::BadDay));
        assert_eq!(
            days("2026-11-05", "2026-11-04", NOW, WIB),
            Err(RangeError::Backwards)
        );
        assert_eq!(
            days("2026-11-24", "2026-11-30", NOW, WIB),
            Err(RangeError::Future)
        );
        assert_eq!(
            days("2025-11-01", "2026-11-02", NOW, WIB),
            Err(RangeError::TooLong)
        );
        assert!(
            days("2025-11-02", "2026-11-02", NOW, WIB).is_ok(),
            "366 days"
        );
    }
}
