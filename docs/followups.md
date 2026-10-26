# Follow-ups

Work skipped during the fast UI round of 2026-10-26 (maintenance and pause). Fix or decide each row, then delete this file.

| Change | Still needs |
| --- | --- |
| Plan maintenance left the monitor and heartbeat pages; it is in the row menu and the batch bar | `e2e/specs/incidents.spec.ts` "pauses a heartbeat, plans maintenance from its page" uses the old button |
| Batch pause, resume and delete (`POST .../monitors/batch`, `.../heartbeats/batch`) | api-specs 04 rows, api-specs 03 section 3.4 key API table, `keyApiDocs.ts`, guide `monitors.md` and `heartbeats.md`, an e2e for batch delete |
| Maintenance moved from `incidents/maintenance` to `maintenance` | guide `incidents.md` paths and screenshots |
