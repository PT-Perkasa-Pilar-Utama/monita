//! Unit tests for the chart geometry.

use super::*;

fn b(index: usize, total: i64, ok: i64, avg_ms: Option<i64>) -> Bucket {
    Bucket {
        index,
        total,
        ok,
        avg_ms,
    }
}

#[test]
fn a_window_ends_with_the_bucket_that_holds_now() {
    let w = Window::new(Range::Day, 10 * 86_400 + 1000, 0);
    assert_eq!(w.step, 900);
    assert_eq!(w.buckets, 96);
    assert_eq!(w.end(), 10 * 86_400 + 1800);
    assert_eq!(
        Window::new(Range::Month, 10 * 86_400 + 5, 0).from,
        (10 - 29) * 86_400
    );
}

#[test]
fn the_y_axis_rounds_up_to_a_readable_top() {
    assert_eq!(nice(0), 100);
    assert_eq!(nice(130), 200);
    assert_eq!(nice(2300), 2500);
    assert_eq!(nice(4100), 5000);
    assert_eq!(ms(2500), "2.5s");
    assert_eq!(ms(250), "250ms");
    assert_eq!(ms(3750), "3.75s");
}

#[test]
fn a_gap_longer_than_the_interval_breaks_the_line_and_failures_are_marked() {
    let w = Window {
        off: 0,
        from: 0,
        step: 60,
        buckets: 60,
    };
    let buckets = [
        b(0, 2, 2, Some(100)),
        b(1, 2, 1, Some(100)),
        b(3, 1, 1, Some(50)),
    ];
    let chart = latency(Range::Hour, &w, &buckets, 1, 0, 3600, &[]);
    assert_eq!(chart.line.matches('M').count(), 2, "{}", chart.line);
    assert_eq!(chart.line.matches('L').count(), 1, "{}", chart.line);
    assert_eq!(chart.area.matches('Z').count(), 2, "{}", chart.area);
    assert_eq!(chart.dots.len(), 3);
    assert_eq!(chart.fails.len(), 1);
    assert!(chart.fails[0].title.contains("1 of 2 checks failed"));
    assert!(!chart.empty);
    let bridged = latency(Range::Hour, &w, &buckets, 2, 0, 3600, &[]);
    assert_eq!(bridged.line.matches('M').count(), 1, "{}", bridged.line);
    assert!(latency(Range::Hour, &w, &[], 1, 0, 3600, &[]).empty);
}

#[test]
fn bands_split_the_covered_time_at_each_down_span() {
    let w = Window {
        off: 0,
        from: 0,
        step: 60,
        buckets: 60,
    };
    let (up, down) = bands(&w, 600, 3600, &[(1200, 1800), (-500, 100)]);
    assert_eq!(down.len(), 1, "a span before the covered time is dropped");
    assert_eq!(up.len(), 2);
    assert!((up[0].x - x_of(&w, 600)).abs() < 0.01);
    assert!(down[0].title.contains("10m"));
}

#[test]
fn a_bucket_with_a_fail_ping_is_red() {
    let w = Window {
        off: 0,
        from: 0,
        step: 900,
        buckets: 96,
    };
    let chart = pings(
        Range::Day,
        &w,
        &[b(2, 1, 1, None), b(5, 2, 1, None), b(7, 0, 0, None)],
        0,
        86_400,
        &[],
    );
    assert_eq!((chart.oks.len(), chart.fails.len()), (1, 1));
    assert!(
        chart.fails[0].h > chart.oks[0].h,
        "two pings stand taller than one"
    );
}

#[test]
fn every_bucket_answers_a_hover_even_an_empty_one() {
    let w = Window::new(Range::Hour, 3600 * 10, 420);
    let c = pings(Range::Hour, &w, &[b(2, 2, 1, None)], 0, 3600 * 10, &[]);
    assert_eq!(c.hovers.len(), w.buckets);
    assert!(
        c.hovers[0].title.ends_with(": No pings"),
        "{}",
        c.hovers[0].title
    );
    assert!(
        c.hovers[2].title.ends_with(": 2 pings, 1 failed"),
        "{}",
        c.hovers[2].title
    );
    // The viewer's own clock (UTC+7 here), with no zone name.
    assert!(
        c.hovers[2].title.starts_with("1970-01-01 16:03"),
        "{}",
        c.hovers[2].title
    );
    assert!(!c.hovers[2].title.contains("UTC"), "{}", c.hovers[2].title);
}
