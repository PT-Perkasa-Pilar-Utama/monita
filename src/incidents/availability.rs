//! Availability of one monitor or heartbeat over fixed periods, and how long it has been up or
//! down. Downtime is automatic incidents only, as on the status page (history.rs).

use super::{ItemRef, store};
use crate::platform::{db::Db, time};

#[derive(serde::Serialize)]
pub struct Period {
    pub label: &'static str,
    /// Percent up, or `None` when the item did not exist yet.
    pub availability: Option<f64>,
    pub downtime_s: i64,
    pub incidents: i64,
    pub longest_s: Option<i64>,
    pub average_s: Option<i64>,
}

/// Up or down since a moment: the end of the last automatic incident, its start while it is
/// open, or the item's creation.
pub struct Streak {
    pub down: bool,
    pub since: i64,
}

pub struct Report {
    pub periods: Vec<Period>,
    pub streak: Streak,
}

/// Today, the last 7, 30 and 365 days, and all time. Each period is one aggregate query.
pub async fn report(db: &Db, item: ItemRef, created_at: i64, now: i64) -> anyhow::Result<Report> {
    let c = db.read().await?;
    let windows = [
        ("Today", time::day_start(now)),
        ("Last 7 days", now - 7 * 86_400),
        ("Last 30 days", now - 30 * 86_400),
        ("Last 365 days", now - 365 * 86_400),
        ("All time", created_at),
    ];
    let mut periods = Vec::with_capacity(windows.len());
    for (label, since) in windows {
        let since = since.max(created_at);
        let (incidents, down, longest_s, average_s) =
            store::period_stats(&c, item.clone(), since, now).await?;
        periods.push(Period {
            label,
            availability: percent_up(down, now - since),
            downtime_s: down,
            incidents,
            longest_s,
            average_s,
        });
    }
    let streak = match store::last_auto(&c, item).await? {
        Some((started, None)) => Streak {
            down: true,
            since: started,
        },
        Some((_, Some(ended))) => Streak {
            down: false,
            since: ended,
        },
        None => Streak {
            down: false,
            since: created_at,
        },
    };
    Ok(Report { periods, streak })
}

fn percent_up(down: i64, span: i64) -> Option<f64> {
    (span > 0).then(|| 100.0 * (1.0 - down.clamp(0, span) as f64 / span as f64))
}

#[cfg(test)]
mod tests {
    use super::percent_up;

    #[test]
    fn availability_is_the_share_of_the_span_not_down() {
        assert_eq!(percent_up(0, 86_400), Some(100.0));
        assert_eq!(percent_up(864, 86_400), Some(99.0));
        assert_eq!(percent_up(90_000, 86_400), Some(0.0), "never below zero");
        assert_eq!(
            percent_up(0, 0),
            None,
            "an item made this second has no span yet"
        );
    }
}
