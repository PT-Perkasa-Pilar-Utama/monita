//! What the monitor and heartbeat detail pages show above their tables, shared by both.

use serde::Serialize;

use crate::{checks::chart::Chart, checks::insight::Insight, incidents::availability::Period};

/// One stat card: label, value, the line under it, and its tone (`up`, `down` or `plain`).
#[derive(Serialize)]
pub struct StatView {
    label: &'static str,
    value: String,
    sub: String,
    tone: &'static str,
}

pub fn stats(raw: [crate::checks::insight::Stat; 4]) -> Vec<StatView> {
    raw.into_iter()
        .map(|(label, value, sub, tone)| StatView {
            label,
            value,
            sub,
            tone,
        })
        .collect()
}

/// The chart for one range, its totals, and availability over fixed periods.
#[derive(Serialize)]
pub struct InsightView {
    title: &'static str,
    /// `hour`, `day`, `week` or `month`.
    range: &'static str,
    total: i64,
    ok: i64,
    mean_ms: Option<i64>,
    chart: Chart,
    periods: Vec<Period>,
}

impl From<Insight> for InsightView {
    fn from(i: Insight) -> Self {
        InsightView {
            title: i.title,
            range: i.series.range.key(),
            total: i.series.total,
            ok: i.series.ok,
            mean_ms: i.series.mean_ms,
            chart: i.series.chart,
            periods: i.report.periods,
        }
    }
}
