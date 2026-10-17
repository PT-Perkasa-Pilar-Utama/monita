# 3. Patterns

## 3.1 Shells

| Shell | Template | Structure |
| --- | --- | --- |
| Admin | `templates/layout/app.html` | `.app` > `.appbar-band` (top bar: brand, project, Docs, Admin menu, account menu; then the project `.tabbar`) + `.content` (`.content--side` with a `.sidenav` for Settings and Admin) > `.pagebar` (crumbs, actions) + the page. Structure follows Beaverflow; colours, type and the magenta underline are Monita's own. |
| Public | `templates/layout/public.html` | `.public` column, 760 px, 15 px body, footer with quiet links |
| Bare | `templates/layout/bare.html` | Sign-in and invite pages: one `.panel` centered, 400 px |

Under 820 px the sidebar turns into a top bar with a horizontal nav. The switcher, counts and footer hide.

## 3.2 Navigation

Project tabs, in this order: Monitors, Heartbeats, Incidents, Tickets, Settings, then Status page at the right end, which leaves the app for the public `/status/{slug}`. Settings holds General, Alerts, API keys and Allowlist in its side navigation. Super admins also get an Admin menu: Users and System (email delivery, server configuration, limits, fixed caps).

## 3.3 Page recipes

| Page | Recipe |
| --- | --- |
| List (monitors, heartbeats, incidents) | `.page-head` (h1 with `.count`) → `.overall` → `.panel` with `.table.table--rich`: each monitor row carries a sparkline and 24-hour uptime, each heartbeat row a deadline bar. Empty: `.empty` in the panel. Primary action in the toolbar. |
| Detail (incident) | `.page-head` → `.split.split--aside`: the facts panel (`.kv`) comes first in the markup and sits in the narrow right column on wide screens; the timeline with its compact composer takes the wide column. |
| Form (create, edit) | `.form-page`: `.panel.panel--form` > `.form`, and a `.form-aside` that explains the rules |
| Projects, users | `.split`: the list in the wide column, the create or invite form in the narrow one |
| Settings, alerts | `.panels`: panels side by side; the API keys table is `.panel--wide` |
| Kanban | `.page-head` with `.tabs` → `.board` |
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
| Put a count beside the title: "Monitors 4" | An uppercase label above a title |
| Put a needed hint in the field's label | A subtitle under a form field |
| Separate facts with spacing or a comma | `·` between facts |
| Use one `--primary` button per view | Put two light pills side by side |
| Separate groups with hairlines | Add shadows to flat panels |
| Fade colors on hover in 160 ms | Pulse, bounce or animate on scroll |
| Write tokens on `:root` | Write a hex value in a template or a component rule |
| Keep secondary text at 4.5:1 or more in both themes | Grey text that fades into the dark background |
