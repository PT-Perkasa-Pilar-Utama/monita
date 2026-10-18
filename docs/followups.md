# Follow-ups

Work deferred during fast frontend cycles. Each line names the change and what it still needs. Delete a line when it is done.

| Commit | Change | Still needs |
| --- | --- | --- |
| feat(projects): page and search the project list on the server | Project list pages 20 at a time; search runs on the server; no project cap | `make e2e` run; an e2e case that pages and searches (the e2e seed has 1 project, so the pager stays hidden) |
| style(web): one corner radius for every control | Buttons and toggles use the input's 8 px corner (`rounded-lg`) | design-system 01/02: button shape ("pill") text |
| feat(web): flat header links, a theme button and a smaller account menu | Users and System are top-bar links; theme is its own button; the account menu holds who you are and Sign out; the header is the card colour | design-system 03 line 20 ("Admin (super admin: Users, System)"); demo snapshot screenshots |
| feat(web): projects page with views, skeletons and a remembered query | Card and list views (remembered), skeletons on every page load, the last `?q=&page=` reopens on a bare visit, new card (owner, created, status page link), stronger hover | `e2e/specs/projects.spec.ts` (card markup changed); design-system 02 rows for `ViewToggle`, `ProjectTile`, `Pager` gap; demo screenshots |
| next | Health per project on the card and the list row (state, open incidents, open tickets) | API: per-project summary on `GET /api/ui/projects`; 20 projects x 4 bounded reads now, one `IN` query later |
| feat(web): the header holds Projects; the rest moves to the account menu | Account menu: Admin settings, Toggle theme, Docs, Sign out | static demo: `e2e/demo/demo.js` toggles the theme through a `Switch to the` button, which no longer exists; design-system 03 top bar row |
| feat(projects): filter the project list by owner and sort it by age | `?owner=&sort=newest|oldest`, `owner_options` for a super admin; Owner and Sort menus in the list band; status page link is an icon button | api-specs 03 list row (`owner`, `sort`, `owner_options`); design-system 02 `FilterMenu`; CLAUDE.md quick reference for `make dev` |
| feat(web): project overview edits in place, with a narrower About column | Edit project is the one header action; About turns into the description and icon form; Save and Discard changes replace it; About is 18rem wide; the Status page tab is gone, About has an Open button | `e2e/specs` that click New monitor or Report incident on the overview, or the Status page tab; design-system 03 overview recipe |
