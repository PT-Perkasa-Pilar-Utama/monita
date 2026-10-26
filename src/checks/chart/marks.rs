//! The marks laid over a chart: up and down bands, and a hover strip per bucket.

use super::{Bucket, FULL, Mark, Window, bucket_x, when, x_of};
use crate::platform::time;

/// Up and down bands from `covered_from` to `now`: down where an automatic incident was open.
pub(super) fn bands(
    w: &Window,
    covered_from: i64,
    now: i64,
    down: &[(i64, i64)],
) -> (Vec<Mark>, Vec<Mark>) {
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

/// One band per maintenance window over the source, clipped to the chart and to `now`.
pub fn held(w: &Window, now: i64, windows: &[(i64, i64, String)]) -> Vec<Mark> {
    let end = now.min(w.end());
    windows
        .iter()
        .map(|(s, e, title)| ((*s).max(w.from), (*e).min(end), title))
        .filter(|(s, e, _)| s < e)
        .map(|(s, e, title)| {
            band(
                w,
                s,
                e,
                &format!("Maintenance: {title}, {}", time::duration(e - s)),
            )
        })
        .collect()
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

/// One hover strip per bucket, empty ones too, so the readout answers anywhere on the chart.
pub(super) fn hovers(
    w: &Window,
    buckets: &[Bucket],
    none: &str,
    say: impl Fn(&Bucket) -> String,
) -> Vec<Mark> {
    (0..w.buckets)
        .map(|i| {
            let (x, bw) = bucket_x(w, i);
            let what = buckets
                .iter()
                .find(|b| b.index == i && b.total > 0)
                .map_or_else(|| none.to_string(), &say);
            Mark {
                x,
                w: bw,
                y: FULL.0,
                h: FULL.1,
                title: format!("{}: {what}", when(w, i)),
            }
        })
        .collect()
}

pub(super) fn plural(n: i64, noun: &str) -> String {
    if n == 1 {
        noun.to_string()
    } else {
        format!("{noun}s")
    }
}
