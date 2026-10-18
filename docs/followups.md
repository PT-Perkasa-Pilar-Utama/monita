# Follow-ups

Work deferred during fast frontend cycles. Each line names the change and what it still needs. Delete a line when it is done.

| Commit | Change | Still needs |
| --- | --- | --- |
| feat(projects): page and search the project list on the server | Project list pages 20 at a time; search runs on the server; no project cap | `make e2e` run; an e2e case that pages and searches (the e2e seed has 1 project, so the pager stays hidden) |
| style(web): one corner radius for every control | Buttons and toggles use the input's 8 px corner (`rounded-lg`) | design-system 01/02: button shape ("pill") text |
| feat(web): flat header links, a theme button and a smaller account menu | Users and System are top-bar links; theme is its own button; the account menu holds who you are and Sign out; the header is the card colour | design-system 03 line 20 ("Admin (super admin: Users, System)"); demo snapshot screenshots |
| feat(web): projects page with views, skeletons and a remembered query | Card and list views (remembered), skeletons on every page load, the last `?q=&page=` reopens on a bare visit, new card (owner, created, status page link), stronger hover | `e2e/specs/projects.spec.ts` (card markup changed); design-system 02 rows for `ViewToggle`, `ProjectTile`, `Pager` gap; demo screenshots |
| next | Health per project on the card and the list row (state, open incidents, open tickets) | API: per-project summary on `GET /api/ui/projects`; 20 projects x 4 bounded reads now, one `IN` query later |
