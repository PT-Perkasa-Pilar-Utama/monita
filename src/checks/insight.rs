//! What the monitor and heartbeat pages show above their tables: the streak, the chart for a
//! range and availability over fixed periods. One view for both, sent by `checks::api`.

use super::{
    chart::Range,
    series::{self, Series},
};
use crate::{
    incidents::{
        ItemRef,
        availability::{self, Report},
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
    /// Minutes east of UTC the page shows times in.
    pub off: i64,
}

impl Insight {
    pub async fn monitor(
        db: &Db,
        m: &super::Monitor,
        range: Range,
        now: i64,
        off: i64,
    ) -> anyhow::Result<Self> {
        let item = ItemRef {
            kind: crate::incidents::SourceKind::Monitor,
            id: m.id,
        };
        Ok(Self {
            title: "Response time",
            series: series::monitor(db, m, range, now, off).await?,
            report: availability::report(db, item, m.created_at, now, off).await?,
            now,
            off,
        })
    }

    pub async fn heartbeat(
        db: &Db,
        hb: &super::heartbeat::Heartbeat,
        range: Range,
        now: i64,
        off: i64,
    ) -> anyhow::Result<Self> {
        let item = ItemRef {
            kind: crate::incidents::SourceKind::Heartbeat,
            id: hb.id,
        };
        Ok(Self {
            title: "Recorded pings",
            series: series::heartbeat(db, hb, range, now, off).await?,
            report: availability::report(db, item, hb.created_at, now, off).await?,
            now,
            off,
        })
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

    pub fn mean(&self) -> String {
        self.series
            .mean_ms
            .map_or("–".into(), |v| format!("{v} ms"))
    }

    /// "Up for 3d 2h", or how old the item is before its first check or ping.
    pub fn streak_stat(&self, seen: bool) -> Stat {
        if !seen {
            let age = time::ago(self.report.streak.since, self.now);
            return ("Created", age, "nothing recorded yet".into(), "plain");
        }
        let (label, value) = self.streak();
        let since = format!("since {}", time::stamp(self.report.streak.since, self.off));
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
}
