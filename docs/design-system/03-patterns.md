# 3. Patterns

## 3.1 Shells

| Shell | Template | Structure |
| --- | --- | --- |
| Admin | `templates/layout/app.html` | `.app` > `.appbar-band` (top bar: brand, project, Projects, Admin menu, Docs, account menu; then the project `.tabbar`) + `.content` (`.content--side` with a `.sidenav` for Settings and Admin) > `nav.crumbs` (detail pages only) + the page, which opens with `.page-head` or `.hero`. The shared `#confirm` dialog closes the layout. Structure follows Beaverflow; colours, type and the magenta underline are Monita's own. |
| Public | `templates/layout/public.html` | `.public` column, 760 px, 15 px body, footer with quiet links |
| Bare | `templates/layout/bare.html` | Sign-in and invite pages: one `.panel` centered, 400 px |

Under 820 px the side navigation turns into a row of links above the page, the area links show icons only, and the tab bar scrolls sideways.

## 3.2 Navigation

Project tabs, in this order: Overview, Monitors, Heartbeats, Incidents, Tickets, Settings, then Status page at the right end, which leaves the app for the public `/status/{slug}`. Settings holds General, Alerts, API keys and Allowlist, each its own page in the side navigation. Super admins also get an Admin menu: Users and System (email delivery, server configuration, limits, fixed caps). Only Settings and Admin have a side navigation; every list stays full width.

## 3.3 Page recipes

| Page | Recipe |
| --- | --- |
| Overview | `.hero` (project badge, name, New monitor and Report incident) → `.split.split--about`: `.tiles` and the open incidents panel with an "All incidents" button in its `.tablebar`, beside `aside.about`, which sticks on wide screens and follows the page on a phone. |
| List (monitors, heartbeats, incidents) | `.page-head` (h1, primary action) → `.overall` → `.panel` with `.tablebar` (count, search, State and Severity) and `.table.table--rich`: each monitor row carries a sparkline and 24-hour uptime, each heartbeat row a deadline bar. Empty: `.empty` in the panel; a filter that hides every row says so and offers Clear the filters. |
| Detail (monitor, heartbeat, incident, ticket) | `nav.crumbs` → `.hero` (title, state and tags, `.page-actions`) → the page. Incident: `.split.split--aside` with the facts panel (`.kv`) first in the markup, in the narrow right column on wide screens; Post update opens a dialog. |
| Form (create, edit) | `nav.crumbs` → `.page-head` (h1) → `.form-page`: `.panel.panel--form` > `.form`, and a `.form-aside` that explains the rules |
| Projects | `.page-head` (search, New project) → `.pcards`; New project is a dialog with Name, Owner (super admin), Description and Icon |
| Users | `.page-head` (Invite tech lead) → `.panel` with `.tablebar` and the table; Invite is a dialog, Disable asks to confirm |
| Settings (General, Alerts, API keys, Allowlist) | Side navigation → `.page-head` → `.panels` (General, Alerts) or one table panel (API keys, with New API key as a dialog) or one form panel (Allowlist) |
| Ticket board | `.page-head` (Public board) → `.board` |
| Status page | h1 → `.headline` (state, systems up) → `.sp` grid: `.sp__systems` (problems first, bars for the first 12, the rest in `.sp__more`) beside `.sp__active` and `.sp__past` (`.inc` cards, folded when resolved) → footer links. History: `.inc` cards under month headings. |

## 3.4 Copy rules

- Name things by what a TL knows: "Ping URL", not "heartbeat token endpoint".
- Buttons say what happens: "Save heartbeat", "Revoke key", "Post update".
- Errors say what to do: "Enter a name, for example nightly-settlement."
- Times: relative in lists ("6m ago"), absolute in UTC on detail pages ("2026-10-16 14:02 UTC").
- Write whole sentences. Do not stack short fragments: "No X, no Y", "Nothing ...", "One X, one Y", or a word, a comma or a period, then another word. An empty state names what will appear or what to do: "Waiting for the first check", not "No checks yet".
- Say what is true and literal. No metaphors: "users see the updates you post", not "keeps the status page honest".
- Durations: `6m 18s`, `2h 4m`, `3d 2h`. Latency: `182ms`. Uptime: two decimals on the status page (`99.91%`), three in the availability table (`99.987%`).

## 3.5 Do and don't

| Do | Don't |
| --- | --- |
| Put state colour in a tone pill: tinted fill, coloured word | Decorative dots, pulsing orbs, dots before labels |
| Put a list's count in its table bar: "4 monitors" | An uppercase label above a title |
| Put a needed hint in the field's label | A subtitle under a form field |
| Separate facts with spacing or a comma | `·` between facts |
| Use one `--primary` button per view | Put two light pills side by side |
| Separate groups with hairlines | Add shadows to flat panels |
| Fade colors on hover in 160 ms | Pulse, bounce or animate on scroll |
| Write tokens on `:root` | Write a hex value in a template or a component rule |
| Keep secondary text at 4.5:1 or more in both themes | Grey text that fades into the dark background |
