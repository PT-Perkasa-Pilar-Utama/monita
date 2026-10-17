//! Charts for the monitor and heartbeat pages, drawn on the server as SVG: no script, nothing the
//! CSP blocks. Inputs are per-bucket aggregates, so a chart never holds more than one range's
//! buckets (168 at most), whatever the check interval.

use crate::platform::time;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Range {
    Hour,
    Day,
    Week,
    Month,
}

impl Range {
    pub const ALL: [Range; 4] = [Range::Hour, Range::Day, Range::Week, Range::Month];

    /// `?range=` value; anything else is a day.
    pub fn parse(raw: Option<&str>) -> Self {
        match raw {
            Some("hour") => Self::Hour,
            Some("week") => Self::Week,
            Some("month") => Self::Month,
            _ => Self::Day,
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Hour => "hour",
            Self::Day => "day",
            Self::Week => "week",
            Self::Month => "month",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Hour => "Hour",
            Self::Day => "Day",
            Self::Week => "Week",
            Self::Month => "Month",
        }
    }

    /// (seconds per bucket, buckets, buckets between x labels).
    fn grid(self) -> (i64, usize, usize) {
        match self {
            Self::Hour => (60, 60, 10),
            Self::Day => (900, 96, 16),
            Self::Week => (3600, 168, 24),
            Self::Month => (86_400, 30, 5),
        }
    }
}

/// The buckets of `range` that end with the one holding `now`, aligned to the bucket size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Window {
    pub from: i64,
    pub step: i64,
    pub buckets: usize,
}

impl Window {
    pub fn new(range: Range, now: i64) -> Self {
        let (step, buckets, _) = range.grid();
        let last = now - now.rem_euclid(step);
        Self {
            from: last - (buckets as i64 - 1) * step,
            step,
            buckets,
        }
    }

    pub fn end(&self) -> i64 {
        self.from + self.step * self.buckets as i64
    }
}

/// One bucket of checks or pings: `total` events, `ok` of them good, and the mean latency.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Bucket {
    pub index: usize,
    pub total: i64,
    pub ok: i64,
    pub avg_ms: Option<i64>,
}

/// A rectangle across the plot, with the hover text it carries.
pub struct Mark {
    pub x: f64,
    pub w: f64,
    pub y: f64,
    pub h: f64,
    pub title: String,
}

const FULL: (f64, f64) = (TOP, BOTTOM - TOP);

pub struct Tick {
    pub at: f64,
    pub label: String,
}

/// What `checks/insight.html` draws. Coordinates are in a 960 x 240 view box.
pub struct Chart {
    pub line: String,
    pub area: String,
    /// (x, y) of each mean, drawn as dots when there are few.
    pub dots: Vec<(f64, f64)>,
    pub up: Vec<Mark>,
    pub down: Vec<Mark>,
    pub fails: Vec<Mark>,
    pub oks: Vec<Mark>,
    pub hovers: Vec<Mark>,
    pub y_ticks: Vec<Tick>,
    pub x_ticks: Vec<Tick>,
    pub empty: bool,
}

pub const LEFT: f64 = 56.0;
pub const RIGHT: f64 = 952.0;
pub const TOP: f64 = 10.0;
pub const BOTTOM: f64 = 212.0;

fn x_of(w: &Window, t: i64) -> f64 {
    let span = (w.end() - w.from) as f64;
    LEFT + (RIGHT - LEFT) * ((t - w.from) as f64 / span).clamp(0.0, 1.0)
}

fn bucket_x(w: &Window, i: usize) -> (f64, f64) {
    let x = x_of(w, w.from + i as i64 * w.step);
    (x, x_of(w, w.from + (i as i64 + 1) * w.step) - x)
}

fn when(w: &Window, i: usize) -> String {
    let t = w.from + i as i64 * w.step;
    if w.step >= 86_400 {
        time::day(t)
    } else {
        time::utc(t)
    }
}

/// The smallest of 1, 2, 2.5 and 5 times a power of ten that holds `v`; 100 at least.
fn nice(v: i64) -> i64 {
    let mut p = 1;
    loop {
        for m in [10, 20, 25, 50] {
            if v <= m * p {
                return (m * p).max(100);
            }
        }
        p *= 10;
    }
}

fn ms(v: i64) -> String {
    if v >= 1000 {
        format!("{}s", v as f64 / 1000.0)
    } else {
        format!("{v}ms")
    }
}

fn x_ticks(range: Range, w: &Window) -> Vec<Tick> {
    let (_, _, every) = range.grid();
    (0..w.buckets)
        .filter(|i| i % every == 0)
        .map(|i| {
            let t = w.from + i as i64 * w.step;
            let stamp = time::utc(t);
            let label = if w.step >= 3600 && range != Range::Day {
                stamp[5..10].to_string()
            } else {
                stamp[11..16].to_string()
            };
            Tick {
                at: x_of(w, t),
                label,
            }
        })
        .collect()
}

/// Up and down bands from `covered_from` to `now`: down where an automatic incident was open.
fn bands(w: &Window, covered_from: i64, now: i64, down: &[(i64, i64)]) -> (Vec<Mark>, Vec<Mark>) {
    let (start, end) = (covered_from.max(w.from), now.min(w.end()));
    let mut ups = Vec::new();
    let mut downs = Vec::new();
    let mut at = start;
    let mut spans: Vec<(i64, i64)> = down
        .iter()
        .map(|&(s, e)| (s.max(start), e.min(end)))
        .filter(|(s, e)| s < e)
        .collect();
    spans.sort_unstable();
    for (s, e) in spans {
        if s > at {
            ups.push(band(w, at, s, "Up"));
        }
        downs.push(band(
            w,
            s.max(at),
            e,
            &format!("Down for {}", time::duration(e - s)),
        ));
        at = at.max(e);
    }
    if end > at {
        ups.push(band(w, at, end, "Up"));
    }
    (ups, downs)
}

fn band(w: &Window, s: i64, e: i64, title: &str) -> Mark {
    let x = x_of(w, s);
    Mark {
        x,
        w: (x_of(w, e) - x).max(1.0),
        y: FULL.0,
        h: FULL.1,
        title: title.to_string(),
    }
}

fn blank(range: Range, w: &Window) -> Chart {
    Chart {
        line: String::new(),
        area: String::new(),
        dots: Vec::new(),
        up: Vec::new(),
        down: Vec::new(),
        fails: Vec::new(),
        oks: Vec::new(),
        hovers: Vec::new(),
        y_ticks: Vec::new(),
        x_ticks: x_ticks(range, w),
        empty: true,
    }
}

/// Mean response time per bucket as a line, failed checks in red. The line bridges up to
/// `max_gap` empty buckets, the check interval in buckets, and breaks where checks stopped.
#[allow(clippy::too_many_arguments)]
pub fn latency(
    range: Range,
    w: &Window,
    buckets: &[Bucket],
    max_gap: usize,
    covered_from: i64,
    now: i64,
    down: &[(i64, i64)],
) -> Chart {
    let mut chart = blank(range, w);
    let top = nice(buckets.iter().filter_map(|b| b.avg_ms).max().unwrap_or(0));
    let y = |v: i64| BOTTOM - (BOTTOM - TOP) * (v as f64 / top as f64).min(1.0);
    chart.y_ticks = (0..=4)
        .map(|k| Tick {
            at: y(top * k / 4),
            label: ms(top * k / 4),
        })
        .collect();
    (chart.up, chart.down) = bands(w, covered_from, now, down);
    let mut prev: Option<usize> = None;
    let mut run_start = 0.0;
    let mut last_x = 0.0;
    for b in buckets.iter().filter(|b| b.index < w.buckets) {
        let (bx, bw) = bucket_x(w, b.index);
        if b.ok < b.total {
            chart.fails.push(Mark {
                x: bx,
                w: bw.max(1.0),
                y: BOTTOM - 6.0,
                h: 6.0,
                title: format!(
                    "{}: {} of {} checks failed",
                    when(w, b.index),
                    b.total - b.ok,
                    b.total
                ),
            });
        }
        chart.hovers.push(Mark {
            x: bx,
            w: bw,
            y: FULL.0,
            h: FULL.1,
            title: format!(
                "{}: {} checks{}",
                when(w, b.index),
                b.total,
                b.avg_ms
                    .map(|v| format!(", mean {v} ms"))
                    .unwrap_or_default()
            ),
        });
        let Some(v) = b.avg_ms else { continue };
        let (px, py) = (bx + bw / 2.0, y(v));
        chart.dots.push((px, py));
        if prev.is_some_and(|p| b.index - p <= max_gap) {
            chart.line += &format!(" L{px:.1} {py:.1}");
            chart.area += &format!(" L{px:.1} {py:.1}");
        } else {
            if prev.is_some() {
                chart.area += &format!(" L{last_x:.1} {BOTTOM:.1} L{run_start:.1} {BOTTOM:.1} Z");
            }
            chart.line += &format!(" M{px:.1} {py:.1}");
            chart.area += &format!(" M{px:.1} {py:.1}");
            run_start = px;
        }
        prev = Some(b.index);
        last_x = px;
    }
    if prev.is_some() {
        chart.area += &format!(" L{last_x:.1} {BOTTOM:.1} L{run_start:.1} {BOTTOM:.1} Z");
        chart.empty = false;
    }
    if chart.dots.len() > 60 {
        chart.dots.clear();
    }
    chart.line = chart.line.trim().to_string();
    chart.area = chart.area.trim().to_string();
    chart
}

/// Up and down time as bands, and a stroke for each bucket that received a ping.
pub fn pings(
    range: Range,
    w: &Window,
    buckets: &[Bucket],
    covered_from: i64,
    now: i64,
    down: &[(i64, i64)],
) -> Chart {
    let mut chart = blank(range, w);
    (chart.up, chart.down) = bands(w, covered_from, now, down);
    // A bar per bucket that heard a ping, as tall as its count against the busiest bucket.
    let most = buckets.iter().map(|b| b.total).max().unwrap_or(1).max(1) as f64;
    for b in buckets
        .iter()
        .filter(|b| b.index < w.buckets && b.total > 0)
    {
        let (bx, bw) = bucket_x(w, b.index);
        let width = (bw * 0.6).max(2.0);
        let h = (FULL.1 * (b.total as f64 / most)).max(FULL.1 * 0.15);
        let mark = Mark {
            x: bx + (bw - width) / 2.0,
            w: width,
            y: BOTTOM - h,
            h,
            title: format!(
                "{}: {} ok, {} failed",
                when(w, b.index),
                b.ok,
                b.total - b.ok
            ),
        };
        if b.ok < b.total {
            chart.fails.push(mark);
        } else {
            chart.oks.push(mark);
        }
    }
    chart.empty = chart.up.is_empty() && chart.down.is_empty();
    chart
}

#[cfg(test)]
mod tests {
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
        let w = Window::new(Range::Day, 10 * 86_400 + 1000);
        assert_eq!(w.step, 900);
        assert_eq!(w.buckets, 96);
        assert_eq!(w.end(), 10 * 86_400 + 1800);
        assert_eq!(
            Window::new(Range::Month, 10 * 86_400 + 5).from,
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
}
