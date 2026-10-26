//! The heartbeat lanes under the plot: ping marks per slot and missed or failed time.

use super::{BeatSeries, LANE_H, LANE_TOP, LANES, Lane, Rect, down_rect, rank, short, slot};
use crate::{checks::chart::Window, platform::time};

pub(super) fn lanes(w: &Window, mut beats: Vec<BeatSeries>) -> Vec<Lane> {
    let rest = beats.split_off(beats.len().min(LANES));
    let mut out: Vec<Lane> = beats
        .iter()
        .enumerate()
        .map(|(n, b)| {
            let mut l = lane(w, n, Some(b.id), short(&b.name), b.state, &[b]);
            l.pinned = b.pinned;
            l
        })
        .collect();
    if !rest.is_empty() {
        let refs: Vec<&BeatSeries> = rest.iter().collect();
        let state = rest
            .iter()
            .map(|b| b.state)
            .min_by_key(|s| rank(s))
            .unwrap_or("up");
        out.push(lane(
            w,
            out.len(),
            None,
            format!("{} more", rest.len()),
            state,
            &refs,
        ));
    }
    out
}

fn lane(
    w: &Window,
    n: usize,
    id: Option<i64>,
    name: String,
    state: &'static str,
    beats: &[&BeatSeries],
) -> Lane {
    let top = LANE_TOP + LANE_H * n as f64;
    let mut marks = Vec::new();
    let mut slots = Vec::new();
    for b in beats {
        for &(s, e) in &b.down {
            let mut r = down_rect(w, s, e, top + 3.0, LANE_H - 6.0, &b.name);
            r.title = format!("{} missed or failed for {}", b.name, time::duration(e - s));
            marks.push(r);
        }
    }
    for i in 0..w.buckets {
        let (ok, total) = beats
            .iter()
            .filter_map(|b| b.buckets.iter().find(|x| x.index == i))
            .fold((0, 0), |(o, t), x| (o + x.ok, t + x.total));
        if total == 0 {
            continue;
        }
        let (x, bw) = slot(w, i);
        let size = 8.0f64.min(bw - 1.0).max(3.0);
        let failed = total - ok;
        let text = if failed > 0 {
            format!("{failed} failed of {total} pings")
        } else {
            format!("{total} ok")
        };
        slots.push((i, if failed > 0 { "fail" } else { "ok" }, text.clone()));
        marks.push(Rect {
            x: x + (bw - size) / 2.0,
            w: size,
            y: top + (LANE_H - size) / 2.0,
            h: size,
            tone: if failed > 0 { "fail" } else { "ok" },
            title: format!("{name}: {text}"),
        });
    }
    Lane {
        id,
        name,
        state,
        pinned: false,
        y: top,
        marks,
        slots,
    }
}
