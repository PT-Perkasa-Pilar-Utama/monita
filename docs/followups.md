# Follow-ups

Work deferred during fast frontend cycles. Each line names what is still open. Delete a line when it is done.

On 2026-10-19 the docs (api-specs, technical-specs, design-system, guide, decision log) and the e2e suite caught up with every earlier row. What remains:

| Area | Still needs |
| --- | --- |
| e2e | Cases that need bulk data or rare state: a board column's Show more (over 25 tickets), a month's "Show all N incidents" on the public Incidents tab (over 3 resolved), the window arrows on Past uptime (needs older months), resolving an automatic incident by hand, the four heartbeat stats, the ticket header's ref ID line, and the overview's New monitor and Report incident buttons |
| Docs | `docs/GLOSSARY.md` does not exist, but `docs/technical-specs/_index.md` and `01-overview.md` link to it. Write it, or drop the links |
| Web | A form keeps a field's error until the next submit in TicketAlertsSection, NewProjectSheet, NewTicketSheet, ProjectAbout, Invite, RateLimits, InviteSheet, Keys, Alerts, General, HeartbeatForm, MonitorForm and the public Ticket tab. Clear it when the field changes (one `dropField` per input) |
| Web | A supervisor sees `(hidden)` for header values, a request body and a sign-in name with no word on why. Say it once on the monitor page |
| Web | The Time zone page does not say that the daily ticket digest stays at its UTC hour |
| API | `PublicPages` has serde defaults for `incidents` and `changelog` but not `known_issues` and `report`, so a PUT that leaves either out is 422 for the whole body |
| API | Revoking an already-revoked ref ID answers 204 (`revoke_ref_of` uses `coalesce`); decide whether that should be 404 |
| API | The public board answers 200 with an empty list for `?stage=triage` but 404 for any other unknown stage |
| Code | `MonitorView.every`'s comment says `30s`, `1m`; `interval_label` returns `30 seconds`, `1 minute`. `save_ticket_alerts` builds `enabled: true` that the INSERT never writes. Both mislead a reader |
