//! The public incidents tab: three months at a time, newest month first.

use serde::Serialize;

use super::{IncidentStatus, ProjectRef};
use crate::{
    incidents::history::Touching,
    platform::{caps, db::Db, now, time},
};

/// Three months ending with the month of `to`, never after the current month, and where the
/// arrows lead. `floor` is the first moment worth showing, such as the project's creation.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Window {
    /// Month starts, oldest first.
    pub starts: [i64; 3],
    /// The start of the month after the last one.
    pub end: i64,
    /// A moment in the last month of the window before, when one has data.
    pub prev_to: Option<i64>,
    /// A moment in the last month of the window after, when this one is not current.
    pub next_to: Option<i64>,
}

pub(super) fn window(to: Option<i64>, now: i64, off: i64, floor: i64) -> Window {
    let current = time::month_start_at(now, off);
    let last = time::month_start_at(to.unwrap_or(now), off).min(current);
    let mid = time::prev_month_at(last, off);
    let first = time::prev_month_at(mid, off);
    let ahead = (0..3).fold(last, |m, _| time::next_month_at(m, off));
    Window {
        starts: [first, mid, last],
        end: time::next_month_at(last, off),
        prev_to: (first > time::month_start_at(floor, off))
            .then(|| time::prev_month_at(first, off)),
        next_to: (last < current).then(|| ahead.min(current)),
    }
}

#[derive(Serialize)]
pub struct Month {
    /// `2026-10`.
    pub month: String,
    pub incidents: Vec<IncidentStatus>,
    /// Set when the month has more than one page: the history page's cursor at the month's end.
    pub more: Option<String>,
}

#[derive(Serialize)]
pub struct Months {
    pub project: ProjectRef,
    /// Newest first.
    pub months: Vec<Month>,
    pub prev_to: Option<i64>,
    pub next_to: Option<i64>,
}

/// None when the project does not exist or turned its Incidents tab off.
pub async fn by_month(db: &Db, slug: &str, to: Option<i64>) -> anyhow::Result<Option<Months>> {
    let Some(it) = super::items(db, slug, 1)
        .await?
        .filter(|it| it.project.pages.incidents)
    else {
        return Ok(None);
    };
    let w = window(to, now(), it.off, it.project.created_at);
    let ends = [w.starts[1], w.starts[2], w.end];
    let mut months = Vec::with_capacity(3);
    for (&start, &end) in w.starts.iter().zip(&ends).rev() {
        let list = super::incidents(
            db,
            &it,
            Touching::Between(start, end),
            caps::PUBLIC_HISTORY_PAGE,
        )
        .await?;
        months.push(Month {
            month: time::month_at(start, it.off),
            more: (list.len() as i64 == caps::PUBLIC_HISTORY_PAGE).then(|| format!("{end}.0")),
            incidents: list,
        });
    }
    Ok(Some(Months {
        project: ProjectRef {
            name: it.project.name.clone(),
            slug: it.project.slug.clone(),
        },
        months,
        prev_to: w.prev_to,
        next_to: w.next_to,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    const AUG_1: i64 = 1_785_542_400;
    const SEP_1: i64 = 1_788_220_800;
    const OCT_1: i64 = 1_790_812_800;
    const NOV_1: i64 = 1_793_491_200;
    const JUN_1: i64 = 1_780_272_000;
    const JUL_1: i64 = 1_782_864_000;

    #[test]
    fn the_window_ends_at_the_current_month_and_stops_at_the_floor() {
        let now = OCT_1 + 15 * 86_400;
        let w = window(None, now, 0, JUN_1);
        assert_eq!(w.starts, [AUG_1, SEP_1, OCT_1]);
        assert_eq!(w.end, NOV_1);
        assert_eq!(w.prev_to, Some(JUL_1), "July ends the window before");
        assert_eq!(w.next_to, None, "this is the current month");
        assert_eq!(
            window(Some(NOV_1 + 86_400), now, 0, JUN_1).starts[2],
            OCT_1,
            "never after now"
        );
        assert_eq!(
            window(None, now, 0, AUG_1 + 5).prev_to,
            None,
            "nothing before the floor"
        );
        let back = window(Some(SEP_1 - 86_400), now, 0, JUN_1);
        assert_eq!(back.starts[2], AUG_1);
        assert_eq!(
            back.next_to,
            Some(OCT_1),
            "three months ahead, clamped to now"
        );
    }
}
