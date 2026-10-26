//! The Overview's one chart (v1.1 Q39): the last 24 hours of a project's monitors and heartbeats on
//! one time axis. The monitors that need the team draw as response-time lines on a log scale, the
//! rest as faint lines behind them, and heartbeats as lanes of ping marks under it. It stays
//! readable at the per-project caps: 5 lines and 6 lanes at most, whatever the project holds.

use serde::Serialize;

use super::{Bucket, Tick, Window};
use crate::platform::time;

/// Lines and lanes drawn on their own; the rest draw faint or fold into the last lane.
pub const LINES: usize = 5;
pub const LANES: usize = 6;

/// The view box: 1000 wide. Names and y labels sit left of the plot.
const PL: f64 = 200.0;
const PR: f64 = 990.0;
const PT: f64 = 10.0;
const PB: f64 = 250.0;
const LANE_TOP: f64 = 266.0;
const LANE_H: f64 = 24.0;

/// One monitor's last day.
pub struct MonitorSeries {
    pub id: i64,
    pub name: String,
    pub state: &'static str,
    pub interval_s: i64,
    pub buckets: Vec<Bucket>,
    /// Automatic incident spans, clipped by the caller to the window.
    pub down: Vec<(i64, i64)>,
}

/// One heartbeat's last day.
pub struct BeatSeries {
    pub id: i64,
    pub name: String,
    pub state: &'static str,
    pub buckets: Vec<Bucket>,
    pub down: Vec<(i64, i64)>,
    /// How much of its period and grace has passed since its last good ping, in percent. A healthy
    /// heartbeat close to its deadline ranks first among the healthy ones.
    pub due: i64,
}

/// One of the other monitors, drawn faint behind the lines.
#[derive(Serialize, Debug, PartialEq)]
pub struct Faint {
    pub id: i64,
    pub name: String,
    pub path: String,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct Line {
    pub id: i64,
    pub name: String,
    pub state: &'static str,
    /// The SVG path, empty without a check in the window.
    pub path: String,
    pub mean_ms: Option<i64>,
    pub checks: i64,
    pub failed: i64,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub w: f64,
    pub y: f64,
    pub h: f64,
    /// `ok`, `fail` or `down`.
    pub tone: &'static str,
    pub title: String,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct Lane {
    /// `None` for the lane that holds the rest.
    pub id: Option<i64>,
    pub name: String,
    pub state: &'static str,
    pub y: f64,
    pub marks: Vec<Rect>,
    /// (slot, tone, text) of each slot that heard a ping, for the readout.
    #[serde(skip)]
    pub slots: Vec<(usize, &'static str, String)>,
}

/// One 15-minute slot, for the hover readout: its x span and a line of text per series.
#[derive(Serialize, Debug, PartialEq)]
pub struct Column {
    pub x: f64,
    pub w: f64,
    pub label: String,
    pub rows: Vec<(String, String, &'static str)>,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct OverviewChart {
    pub height: f64,
    pub plot: [f64; 4],
    pub lines: Vec<Line>,
    /// The other monitors, faint.
    pub others: Vec<Faint>,
    pub down: Vec<Rect>,
    pub lanes: Vec<Lane>,
    pub y_ticks: Vec<Tick>,
    pub x_ticks: Vec<Tick>,
    pub columns: Vec<Column>,
    /// Monitors and heartbeats left out because they are paused.
    pub paused: usize,
}

/// Down first, then an issue, maintenance, waiting, up, and paused last.
pub fn rank(state: &str) -> u8 {
    match state {
        "down" => 0,
        "issue" => 1,
        "maintenance" => 2,
        "unknown" => 3,
        "up" => 4,
        _ => 5,
    }
}

fn x_at(w: &Window, t: i64) -> f64 {
    let span = (w.end() - w.from) as f64;
    PL + (PR - PL) * ((t - w.from) as f64 / span).clamp(0.0, 1.0)
}

pub(super) fn slot(w: &Window, i: usize) -> (f64, f64) {
    let x = x_at(w, w.from + i as i64 * w.step);
    (x, x_at(w, w.from + (i as i64 + 1) * w.step) - x)
}

fn totals(b: &[Bucket]) -> (i64, i64, Option<i64>) {
    let checks: i64 = b.iter().map(|b| b.total).sum();
    let failed: i64 = b.iter().map(|b| b.total - b.ok).sum();
    let weighted: i64 = b.iter().filter_map(|b| b.avg_ms.map(|v| v * b.total)).sum();
    let timed: i64 = b
        .iter()
        .filter(|b| b.avg_ms.is_some())
        .map(|b| b.total)
        .sum();
    (checks, failed, (timed > 0).then(|| weighted / timed))
}

/// The monitors that need the team first: by state, then the most failed checks, then the
/// slowest, then by name. One with no check in the window draws nothing, so unless it is down or
/// has an issue it goes after the ones that do.
pub fn order_monitors(v: &mut [MonitorSeries]) {
    v.sort_by_cached_key(|m| {
        let (_, failed, mean) = totals(&m.buckets);
        let state = rank(m.state);
        (
            if m.buckets.is_empty() && state > 1 {
                5
            } else {
                state
            },
            -failed,
            -mean.unwrap_or(-1),
            m.name.to_lowercase(),
        )
    });
}

/// Heartbeats by state, then the most trouble (failed pings and time down), then the closest to
/// its deadline, then by name.
pub fn order_beats(v: &mut [BeatSeries]) {
    v.sort_by_cached_key(|b| {
        let (_, failed, _) = totals(&b.buckets);
        let down: i64 = b.down.iter().map(|(s, e)| e - s).sum();
        (rank(b.state), -failed, -down, -b.due, b.name.to_lowercase())
    });
}

/// A log scale over `lo..=hi` milliseconds: (y of a value, ticks).
fn scale(lo: i64, hi: i64) -> (impl Fn(i64) -> f64, Vec<Tick>) {
    let floor = 10f64.powf((lo.max(1) as f64).log10().floor());
    let mut ceil = 10f64.powf((hi.max(1) as f64).log10().ceil());
    if ceil <= floor {
        ceil = floor * 10.0;
    }
    let (a, b) = (floor.log10(), ceil.log10());
    let y =
        move |v: i64| PB - (PB - PT) * (((v.max(1) as f64).log10() - a) / (b - a)).clamp(0.0, 1.0);
    let decades = (b - a).round() as i64;
    let steps: &[f64] = if decades <= 2 {
        &[1.0, 2.0, 5.0]
    } else {
        &[1.0]
    };
    let mut ticks = Vec::new();
    let mut d = floor;
    while d <= ceil + 0.5 {
        for m in steps {
            let v = (d * m).round() as i64;
            if (v as f64) <= ceil + 0.5 {
                ticks.push(Tick {
                    at: y(v),
                    label: ms(v),
                });
            }
        }
        d *= 10.0;
    }
    (y, ticks)
}

fn ms(v: i64) -> String {
    if v >= 1000 {
        format!("{}s", v as f64 / 1000.0)
    } else {
        format!("{v}ms")
    }
}

pub(super) fn short(name: &str) -> String {
    let mut s: String = name.chars().take(22).collect();
    if name.chars().count() > 22 {
        s.push('…');
    }
    s
}

/// Lays out the chart. `monitors` and `beats` come in any order; paused ones are left out.
pub fn overview(w: &Window, monitors: Vec<MonitorSeries>, beats: Vec<BeatSeries>) -> OverviewChart {
    let paused = monitors.iter().filter(|m| m.state == "paused").count()
        + beats.iter().filter(|b| b.state == "paused").count();
    let mut monitors: Vec<MonitorSeries> = monitors
        .into_iter()
        .filter(|m| m.state != "paused")
        .collect();
    let mut beats: Vec<BeatSeries> = beats.into_iter().filter(|b| b.state != "paused").collect();
    order_monitors(&mut monitors);
    order_beats(&mut beats);
    let others = monitors.split_off(monitors.len().min(LINES));

    let means = monitors
        .iter()
        .chain(&others)
        .flat_map(|m| m.buckets.iter().filter_map(|b| b.avg_ms));
    let (lo, hi) = means.fold((i64::MAX, 0), |(lo, hi), v| (lo.min(v), hi.max(v)));
    let (y, y_ticks) = if hi == 0 {
        scale(10, 1000)
    } else {
        scale(lo, hi)
    };

    let lines = monitors.iter().map(|m| line(w, m, &y)).collect();
    let faint = others
        .iter()
        .map(|m| Faint {
            id: m.id,
            name: m.name.clone(),
            path: line(w, m, &y).path,
        })
        .filter(|f| !f.path.is_empty())
        .collect();
    let down = monitors
        .iter()
        .chain(&others)
        .flat_map(|m| m.down.iter().map(move |&(s, e)| (m, s, e)))
        .map(|(m, s, e)| down_rect(w, s, e, PT, PB - PT, &m.name))
        .collect();
    let lanes = lanes(w, beats.split_off(0));
    let height = LANE_TOP + LANE_H * lanes.len() as f64 + 22.0;
    // A label every 4 hours, on the hour in the admin's time zone.
    let x_ticks = (0..w.buckets)
        .map(|i| w.from + i as i64 * w.step)
        .filter(|t| (t + w.off * 60).rem_euclid(4 * 3600) == 0)
        .map(|t| Tick {
            at: x_at(w, t),
            label: time::local(t, w.off)[11..16].to_string(),
        })
        .collect();
    let columns = columns(w, &monitors, &others, &lanes);
    OverviewChart {
        height,
        plot: [PL, PT, PR, PB],
        lines,
        others: faint,
        down,
        lanes,
        y_ticks,
        x_ticks,
        columns,
        paused,
    }
}

fn line(w: &Window, m: &MonitorSeries, y: &impl Fn(i64) -> f64) -> Line {
    let gap = ((m.interval_s + w.step - 1) / w.step).max(1) as usize;
    let mut path = String::new();
    let mut prev: Option<usize> = None;
    for b in m.buckets.iter().filter(|b| b.index < w.buckets) {
        let Some(v) = b.avg_ms else { continue };
        let (x, bw) = slot(w, b.index);
        let cmd = if prev.is_some_and(|p| b.index - p <= gap) {
            'L'
        } else {
            'M'
        };
        path += &format!("{cmd}{:.1} {:.1} ", x + bw / 2.0, y(v));
        prev = Some(b.index);
    }
    let (checks, failed, mean_ms) = totals(&m.buckets);
    Line {
        id: m.id,
        name: m.name.clone(),
        state: m.state,
        path: path.trim().to_string(),
        mean_ms,
        checks,
        failed,
    }
}

pub(super) fn down_rect(w: &Window, s: i64, e: i64, top: f64, h: f64, name: &str) -> Rect {
    let x = x_at(w, s.max(w.from));
    Rect {
        x,
        w: (x_at(w, e.min(w.end())) - x).max(1.5),
        y: top,
        h,
        tone: "down",
        title: format!("{name} down for {}", time::duration(e - s)),
    }
}

mod columns;
mod lanes;
use columns::columns;
use lanes::lanes;

#[cfg(test)]
mod tests;
