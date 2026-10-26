# Follow-ups

Work skipped during the fast UI round of 2026-10-26 (maintenance and pause). Fix or decide each row, then delete this file.

| Change | Still needs |
| --- | --- |
| Plan maintenance left the monitor and heartbeat pages; it is in the row menu and the batch bar | `e2e/specs/incidents.spec.ts` "pauses a heartbeat, plans maintenance from its page" uses the old button |
| Batch pause, resume and delete (`POST .../monitors/batch`, `.../heartbeats/batch`) | api-specs 04 rows, api-specs 03 section 3.4 key API table, `keyApiDocs.ts`, guide `monitors.md` and `heartbeats.md`, an e2e for batch delete |
| Maintenance moved from `incidents/maintenance` to `maintenance` | guide `incidents.md` paths and screenshots |
| Reclassify an incident (planned work, false alarm, count again), migration 0044, `PUT .../incidents/{id}/class` | decision Q46 in `docs/decisions/2026-10-19-v1.1-scope.md`, api-specs 05, technical-specs 06 and 12, guide `incidents.md`, CHANGELOG, key API docs, an e2e |
| Failures inside a maintenance window count as held (`in_window`, migration 0045) and charts draw the window as stripes | technical-specs 06 and 13, guide `monitors.md`, `heartbeats.md` and `incidents.md`, CHANGELOG |
| Deleted monitors and heartbeats leave the project list chart and the pulse bars | CHANGELOG |
| Paused and maintenance stamps, striped rows, the Maintenance tab with Paused now | design-system page recipes, guide screenshots |
