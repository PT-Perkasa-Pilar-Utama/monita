//! The hover readout: a column per slot with a row for each line, the rest, and each lane.

use super::{Column, Lane, MonitorSeries, slot};
use crate::{
    checks::chart::{Bucket, Window},
    platform::time,
};

pub(super) fn columns(
    w: &Window,
    shown: &[MonitorSeries],
    others: &[MonitorSeries],
    lanes: &[Lane],
) -> Vec<Column> {
    (0..w.buckets)
        .map(|i| {
            let (x, bw) = slot(w, i);
            let start = w.from + i as i64 * w.step;
            // A monitor can be down with no check in a slot, when Monita itself was not
            // checking; the readout says so instead of a bare "no check" under the red band.
            let down_here =
                |m: &MonitorSeries| m.down.iter().any(|&(s, e)| s < start + w.step && e > start);
            let mut rows = Vec::new();
            for m in shown {
                let b = m.buckets.iter().find(|b| b.index == i);
                let (text, tone) = match b {
                    Some(b) if b.ok < b.total => {
                        (format!("{} of {} failed", b.total - b.ok, b.total), "fail")
                    }
                    Some(Bucket {
                        avg_ms: Some(v), ..
                    }) => (format!("{v} ms"), "ok"),
                    _ if down_here(m) => ("down, no check".to_string(), "fail"),
                    _ => ("no check".to_string(), "none"),
                };
                rows.push((m.name.clone(), text, tone));
            }
            let vals: Vec<i64> = others
                .iter()
                .filter_map(|m| {
                    m.buckets
                        .iter()
                        .find(|b| b.index == i)
                        .and_then(|b| b.avg_ms)
                })
                .collect();
            if let (Some(lo), Some(hi)) = (vals.iter().min(), vals.iter().max()) {
                rows.push((
                    format!("{} other monitors", others.len()),
                    format!("{lo} to {hi} ms"),
                    "ok",
                ));
            }
            let down = others.iter().filter(|m| down_here(m)).count();
            if down > 0 {
                rows.push((
                    format!("{down} of {} other monitors", others.len()),
                    "down".to_string(),
                    "fail",
                ));
            }
            for l in lanes {
                if let Some((_, tone, text)) = l.slots.iter().find(|(at, _, _)| *at == i) {
                    rows.push((l.name.clone(), text.clone(), tone));
                }
            }
            Column {
                x,
                w: bw,
                label: format!(
                    "{} to {}",
                    &time::local(start, w.off)[11..16],
                    &time::local(start + w.step, w.off)[11..16]
                ),
                rows,
            }
        })
        .collect()
}
