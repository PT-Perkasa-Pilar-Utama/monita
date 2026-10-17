//! Hard caps. Every list a page, an API or a job reads has a bound (CLAUDE.md, Architecture laws).
//! A list people add to is capped at insert, and its read uses the same number as its LIMIT, so a
//! page never hides a row. Spec 08 section 8.2 lists them.

/// Projects in one instance.
pub const PROJECTS: i64 = 200;
/// Accounts in one instance, super admins and TLs.
pub const USERS: i64 = 500;
pub const MONITORS_PER_PROJECT: i64 = 100;
pub const HEARTBEATS_PER_PROJECT: i64 = 100;
pub const MONITORS_FULL: &str = "This project has 100 monitors, its limit. Delete one first.";
pub const HEARTBEATS_FULL: &str = "This project has 100 heartbeats, its limit. Delete one first.";
/// Monitors and heartbeats together: what an incident can name.
pub const SOURCES_PER_PROJECT: i64 = MONITORS_PER_PROJECT + HEARTBEATS_PER_PROJECT;
/// Live API keys per project. Settings also shows the newest revoked ones, up to this many.
pub const KEYS_PER_PROJECT: i64 = 20;
/// Addresses in one alert route.
pub const EMAILS_PER_ROUTE: usize = 20;
/// Status updates posted on one incident.
pub const UPDATES_PER_INCIDENT: i64 = 50;
/// Manual incidents open at once in one project.
pub const OPEN_MANUAL_PER_PROJECT: i64 = 20;
/// Rows of open incident items in one project: one automatic incident per source, plus the
/// open manual ones naming every source.
pub const OPEN_ITEMS: i64 = SOURCES_PER_PROJECT * (1 + OPEN_MANUAL_PER_PROJECT);
/// Tickets a kanban or public board column shows, newest first; the column header counts all.
pub const TICKETS_PER_COLUMN: i64 = 200;
/// Tickets one digest email lists; it says how many more wait in Triage.
pub const DIGEST_LINES: i64 = 50;
/// Status updates per incident on the public page, newest first.
pub const PUBLIC_UPDATES: i64 = 10;
/// Automatic-incident spans read for one project's window. A source that flaps faster than this
/// has its oldest spans left out of the uptime figure.
pub const SPANS: i64 = 10_000;
/// Rows one background job reads or deletes per transaction; it loops until a batch is short.
pub const JOB_BATCH: i64 = 500;
/// Missed heartbeats handled per write: each runs the incident engine and queues alerts.
pub const MISSED_BATCH: i64 = 50;
/// Pings per page on the heartbeat page.
pub const PINGS_PAGE: i64 = 25;

#[cfg(test)]
mod tests {
    #[test]
    fn each_full_message_names_its_cap() {
        assert!(super::MONITORS_FULL.contains(&super::MONITORS_PER_PROJECT.to_string()));
        assert!(super::HEARTBEATS_FULL.contains(&super::HEARTBEATS_PER_PROJECT.to_string()));
    }
}
