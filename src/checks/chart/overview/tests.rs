use super::*;
use crate::checks::chart::Range;

const NOW: i64 = 1_792_108_800 + 12 * 3600;

fn window() -> Window {
    Window::new(Range::Day, NOW, 0)
}

fn checks(ms: i64, failed_at: Option<usize>) -> Vec<Bucket> {
    (0..96)
        .map(|index| Bucket {
            index,
            total: 1,
            ok: i64::from(failed_at != Some(index)),
            avg_ms: Some(ms),
        })
        .collect()
}

fn monitor(id: i64, state: &'static str, ms: i64, failed_at: Option<usize>) -> MonitorSeries {
    MonitorSeries {
        id,
        name: format!("m{id}"),
        state,
        interval_s: 60,
        buckets: checks(ms, failed_at),
        down: Vec::new(),
    }
}

fn beat(id: i64, state: &'static str, fail_at: Option<usize>) -> BeatSeries {
    BeatSeries {
        id,
        name: format!("b{id}"),
        state,
        buckets: (0..96)
            .step_by(4)
            .map(|index| Bucket {
                index,
                total: 1,
                ok: i64::from(fail_at != Some(index)),
                avg_ms: None,
            })
            .collect(),
        down: Vec::new(),
        due: 0,
    }
}

#[test]
fn fifty_monitors_draw_five_lines_and_the_rest_faint() {
    let mut ms: Vec<MonitorSeries> = (1..=50).map(|i| monitor(i, "up", 10 + i, None)).collect();
    ms.push(monitor(99, "down", 5, None));
    ms.push(monitor(98, "paused", 5, None));
    let c = overview(&window(), ms, Vec::new());
    let ids: Vec<i64> = c.lines.iter().map(|l| l.id).collect();
    assert_eq!(ids, [99, 50, 49, 48, 47], "down first, then the slowest");
    assert_eq!(c.others.len(), 46);
    assert!(c.others.iter().all(|f| f.path.starts_with('M')));
    assert_eq!(c.paused, 1, "a paused monitor is left out and counted");
    assert_eq!(c.columns.len(), 96);
    assert_eq!(c.columns[0].rows.len(), 6, "five lines and the band");
}

#[test]
fn failed_checks_rank_before_speed_among_monitors_that_are_up() {
    let c = overview(
        &window(),
        vec![monitor(1, "up", 900, None), monitor(2, "up", 20, Some(3))],
        Vec::new(),
    );
    assert_eq!(c.lines[0].id, 2);
    assert_eq!((c.lines[0].checks, c.lines[0].failed), (96, 1));
    assert_eq!(c.columns[3].rows[0].2, "fail");
}

#[test]
fn a_latency_scale_is_logarithmic_so_fast_and_slow_checks_both_read() {
    let c = overview(
        &window(),
        vec![monitor(1, "up", 5, None), monitor(2, "up", 900, None)],
        Vec::new(),
    );
    let labels: Vec<&str> = c.y_ticks.iter().map(|t| t.label.as_str()).collect();
    assert_eq!(labels, ["1ms", "10ms", "100ms", "1s"]);
    let mid = |path: &str| -> f64 { path.split(' ').nth(1).unwrap().parse().unwrap() };
    let (fast, slow) = (mid(&c.lines[1].path), mid(&c.lines[0].path));
    assert!(fast > slow, "a slower monitor sits higher");
    assert!(
        (PB - fast) > 40.0,
        "the fast one is not flattened onto the axis"
    );
}

#[test]
fn heartbeats_fill_six_lanes_and_one_more_lane_that_keeps_their_failures() {
    let mut bs: Vec<BeatSeries> = (1..=10).map(|i| beat(i, "up", None)).collect();
    bs.push(beat(20, "down", Some(8)));
    bs[0] = beat(1, "up", Some(12));
    let c = overview(&window(), Vec::new(), bs);
    assert_eq!(c.lanes.len(), 7);
    assert_eq!(c.lanes[0].id, Some(20), "down first");
    assert_eq!(c.lanes[1].id, Some(1), "then failed pings");
    assert_eq!(c.lanes[6].id, None);
    assert_eq!(c.lanes[6].name, "5 more");
    let fail = |l: &Lane| l.marks.iter().filter(|m| m.tone == "fail").count();
    assert_eq!(fail(&c.lanes[0]), 1);
    assert_eq!(c.height, LANE_TOP + LANE_H * 7.0 + 22.0);
    assert!(
        c.columns[8]
            .rows
            .iter()
            .any(|r| r.0 == "b20" && r.2 == "fail"),
        "the readout names the failure"
    );
}

#[test]
fn an_empty_project_still_draws_an_axis() {
    let c = overview(&window(), Vec::new(), Vec::new());
    assert!(c.lines.is_empty() && c.lanes.is_empty() && c.others.is_empty());
    let labels: Vec<&str> = c.x_ticks.iter().map(|t| t.label.as_str()).collect();
    assert_eq!(
        labels,
        ["16:00", "20:00", "00:00", "04:00", "08:00", "12:00"],
        "on the hour"
    );
    assert!(!c.y_ticks.is_empty());
}

#[test]
fn among_healthy_heartbeats_the_one_closest_to_its_deadline_comes_first() {
    let mut a = beat(1, "up", None);
    a.due = 20;
    let mut b = beat(2, "up", None);
    b.due = 90;
    let c = overview(&window(), Vec::new(), vec![a, b]);
    assert_eq!(c.lanes[0].id, Some(2));
}

#[test]
fn a_series_with_no_check_gives_its_line_to_one_that_has_data_unless_it_is_down() {
    let mut empty = monitor(1, "unknown", 10, None);
    empty.buckets.clear();
    let mut down = monitor(2, "down", 10, None);
    down.buckets.clear();
    let c = overview(
        &window(),
        vec![empty, down, monitor(3, "up", 10, None)],
        Vec::new(),
    );
    let ids: Vec<i64> = c.lines.iter().map(|l| l.id).collect();
    assert_eq!(ids, [2, 3, 1]);
}
