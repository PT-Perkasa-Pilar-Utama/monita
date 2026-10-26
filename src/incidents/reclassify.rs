//! Reclassify an incident (v1.1 Q46): planned work (maintenance, or a pause that came too late)
//! or a false alarm. A reclassified incident leaves uptime, the charts and the public page, and
//! the team still sees it with who changed it. Restore counts it again but never reopens it.
//!
//! | From | To | Guard |
//! | --- | --- | --- |
//! | counted | planned, false_alarm | an open one closes too, an automatic one with its resolved alert |
//! | planned, false_alarm | the other one | none |
//! | planned, false_alarm | counted | none; it stays closed |

use super::{Ended, store, updates::resolve_by_hand};
use crate::platform::db::Db;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    /// Counts in uptime, the charts and the public page, as every incident starts.
    Counted,
    Planned,
    FalseAlarm,
}

impl Class {
    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "counted" => Some(Self::Counted),
            "planned" => Some(Self::Planned),
            "false_alarm" => Some(Self::FalseAlarm),
            _ => None,
        }
    }

    fn column(self) -> Option<&'static str> {
        match self {
            Self::Counted => None,
            Self::Planned => Some("planned"),
            Self::FalseAlarm => Some("false_alarm"),
        }
    }
}

/// Sets the class of an incident in the project. `by` is the name the team sees beside it.
/// False when the incident is not in the project.
#[allow(clippy::too_many_arguments)]
pub async fn reclassify(
    db: &Db,
    base_url: &str,
    project_id: i64,
    incident_id: i64,
    class: Class,
    by: &str,
    author: &super::Author,
    now: i64,
) -> anyhow::Result<bool> {
    db.write(async |c| {
        let Some((origin, resolved_at)) = store::origin_and_end(c, project_id, incident_id).await?
        else {
            return Ok(false);
        };
        let closing = resolved_at.is_none() && class != Class::Counted;
        if closing {
            let message = match class {
                Class::Planned => "Closed as planned work, so it does not count as downtime.",
                _ => "Closed as a false alarm, so it does not count as downtime.",
            };
            // One closing note even at the update cap: it is what closes a manual incident, and
            // an incident closes once, so the list grows by one at most.
            store::add_update(
                c,
                incident_id,
                &origin,
                resolved_at,
                "resolved",
                message,
                Some(author),
                now,
            )
            .await?;
            if origin == "auto" {
                let ended = if class == Class::Planned {
                    Ended::Planned
                } else {
                    Ended::FalseAlarm
                };
                if let Some(ev) = resolve_by_hand(c, project_id, incident_id, now, ended).await? {
                    crate::notify::send(c, &ev, base_url).await?;
                }
            }
        }
        store::set_class(c, incident_id, class.column(), by, now).await?;
        Ok(true)
    })
    .await
}
