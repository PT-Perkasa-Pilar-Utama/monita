# Follow-ups

Work skipped during the fast UI round of 2026-10-26 (maintenance and pause). Fix or decide each row, then delete this file.

| Change | Still needs |
| --- | --- |
| Plan maintenance left the monitor and heartbeat pages; it is in the row menu and the batch bar | `e2e/specs/incidents.spec.ts:181` ("pauses a heartbeat, plans maintenance from its page") clicks the removed button, so the `flows` shard fails until it changes; `:124` reaches Maintenance through a link that is now the tab, check it still passes |
| Batch pause, resume and delete (`POST .../monitors/batch`, `.../heartbeats/batch`) | api-specs 04 rows, api-specs 03 section 3.4 key API table, `keyApiDocs.ts`, guide `monitors.md` and `heartbeats.md`, an e2e for batch delete |
| Maintenance moved from `incidents/maintenance` to `maintenance` | guide `incidents.md` paths and screenshots; only `incidents/maintenance` redirects, `incidents/maintenance/new` no longer exists |
| Reclassify an incident (planned work, false alarm, count again), migration 0044, `PUT .../incidents/{id}/class` | decision Q46 in `docs/decisions/2026-10-19-v1.1-scope.md`, api-specs 05, technical-specs 06 and 12, guide `incidents.md`, CHANGELOG, key API docs, the CLAUDE.md checklist, a Planned work or False alarm tag on the cross-project Incidents page, an e2e |
| Failures inside a maintenance window count as held (`in_window`, migration 0045) and charts draw the window as stripes | technical-specs 06 and 13, guide `monitors.md`, `heartbeats.md` and `incidents.md`, CHANGELOG, the CLAUDE.md Pause and Maintenance checklist lines |
| Deleted monitors and heartbeats leave the project list chart and the pulse bars | CHANGELOG |
| Paused and maintenance stamps, striped rows, the Maintenance tab with Paused now | design-system page recipes, guide screenshots |
| Who sees it is a card choice under the name in the monitor and heartbeat forms | `e2e/specs/status.spec.ts` passes; guide `monitors.md` and `heartbeats.md` field lists |
| Maintenance and Changelog guides | guide `changelog.md` still says a new project has the public Changelog tab off, which contradicts decision Q28 |
| Locations: setup panel, New token, `monita probe install`, long polling | `scripts/smoke-cli.sh` running `probe install` on a real Linux host in CI; `e2e/specs/admin.spec.ts` location test updated but not run; a decision row for the setup flow and long polling |
| `e2e/specs/tickets.spec.ts:71` fails on the drag to In progress: no "Moved #1 to In progress." toast. It fails on the bundle before the public split too | find the cause and fix the board or the test |
| Charts read 15-minute slots (migration 0046), JSON goes out as zstd, the bundle as br or gzip, status pages load only the public app | CHANGELOG |
