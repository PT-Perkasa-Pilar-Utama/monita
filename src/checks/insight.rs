//! What the monitor and heartbeat pages show above their tables: the streak, the chart for a
//! range and availability over fixed periods. One view for both, in `checks/insight.html`.

use super::{
    chart::Range,
    series::{self, Series},
};
use crate::{
    incidents::{
        ItemRef,
        availability::{self, Period, Report},
    },
    platform::{db::Db, time},
};

/// One stat card: label, value, the line under it, and its tone (`up`, `down` or `plain`).
pub type Stat = (&'static str, String, String, &'static str);

pub struct Insight {
    pub title: &'static str,
    pub series: Series,
    pub report: Report,
    pub now: i64,
}

impl Insight {
    pub async fn monitor(
        db: &Db,
        m: &super::Monitor,
        range: Range,
        now: i64,
    ) -> anyhow::Result<Self> {
        let item = ItemRef {
            kind: crate::incidents::SourceKind::Monitor,
            id: m.id,
        };
        Ok(Self {
            title: "Response time",
            series: series::monitor(db, m, range, now).await?,
            report: availability::report(db, item, m.created_at, now).await?,
            now,
        })
    }

    pub async fn heartbeat(
        db: &Db,
        hb: &super::heartbeat::Heartbeat,
        range: Range,
        now: i64,
    ) -> anyhow::Result<Self> {
        let item = ItemRef {
            kind: crate::incidents::SourceKind::Heartbeat,
            id: hb.id,
        };
        Ok(Self {
            title: "Recorded pings",
            series: series::heartbeat(db, hb, range, now).await?,
            report: availability::report(db, item, hb.created_at, now).await?,
            now,
        })
    }

    pub fn ranges(&self) -> [Range; 4] {
        Range::ALL
    }

    pub fn is_range(&self, r: &Range) -> bool {
        *r == self.series.range
    }

    /// ("Up for" or "Down for", how long).
    pub fn streak(&self) -> (&'static str, String) {
        let s = &self.report.streak;
        let label = if s.down { "Down for" } else { "Up for" };
        (label, time::duration(self.now - s.since))
    }

    pub fn streak_kind(&self) -> &'static str {
        if self.report.streak.down {
            "down"
        } else {
            "up"
        }
    }

    pub fn pct(&self, p: &Period) -> String {
        p.availability.map_or("–".into(), |a| format!("{a:.3}%"))
    }

    /// How the figure reads at a glance: three nines and up is good.
    pub fn grade(&self, p: &Period) -> &'static str {
        match p.availability {
            None => "none",
            Some(a) if a >= 99.9 => "good",
            Some(a) if a >= 99.0 => "fair",
            Some(_) => "poor",
        }
    }

    /// The bar under the figure: 90 % and below is empty, so the last nines stay visible.
    pub fn bar(&self, p: &Period) -> String {
        let a = p.availability.unwrap_or(0.0);
        format!("{:.1}", ((a - 90.0) * 10.0).clamp(0.0, 100.0))
    }

    pub fn dur(&self, s: &Option<i64>) -> String {
        s.filter(|s| *s > 0).map_or("none".into(), time::duration)
    }

    pub fn downtime(&self, p: &Period) -> String {
        self.dur(&Some(p.downtime_s))
    }

    pub fn mean(&self) -> String {
        self.series
            .mean_ms
            .map_or("–".into(), |v| format!("{v} ms"))
    }

    pub fn failed(&self) -> i64 {
        self.series.total - self.series.ok
    }

    /// "Up for 3d 2h", or how old the item is before its first check or ping.
    pub fn streak_stat(&self, seen: bool) -> Stat {
        if !seen {
            let age = time::ago(self.report.streak.since, self.now);
            return ("Created", age, "nothing recorded yet".into(), "plain");
        }
        let (label, value) = self.streak();
        let since = format!("since {}", time::utc(self.report.streak.since));
        (label, value, since, self.streak_kind())
    }

    /// Incidents in the last 30 days, with the longest.
    pub fn incidents_stat(&self) -> Stat {
        let Some(p) = self.report.periods.get(2) else {
            return ("Incidents, 30 days", "0".into(), String::new(), "plain");
        };
        let sub = match p.longest_s {
            Some(s) if p.incidents > 0 => format!("longest {}", time::duration(s)),
            _ => "none in 30 days".into(),
        };
        let tone = if p.incidents > 0 { "down" } else { "up" };
        ("Incidents, 30 days", p.incidents.to_string(), sub, tone)
    }

    pub fn range_word(&self) -> String {
        format!("last {}", self.series.range.label().to_lowercase())
    }

    pub fn f(&self, v: &f64) -> String {
        format!("{v:.1}")
    }
}
