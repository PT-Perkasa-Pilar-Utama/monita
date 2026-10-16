# 3. Patterns

## 3.1 Shells

| Shell | Template | Structure |
| --- | --- | --- |
| Admin | `templates/layout/app.html` | `.app` > `.sidebar` (brand, project switcher, nav, user footer) + `.main` > `.toolbar` (crumbs, actions) + `.content` |
| Public | `templates/layout/public.html` | `.public` column, 760 px, 15 px body, footer with quiet links |
| Bare | `templates/layout/bare.html` | Sign-in and invite pages: one `.panel` centered, 400 px |

Under 820 px the sidebar turns into a top bar with a horizontal nav. The switcher, counts and footer hide.

## 3.2 Navigation

Project menu, in this order: Monitors, Heartbeats, Incidents, Tickets, Settings. Super admins also see an "Admin" group: Users, Settings (limits). Counts on the right in `.nav__count`; a down count uses `.nav__count--alert`.

## 3.3 Page recipes

| Page | Recipe |
| --- | --- |
| List (monitors, heartbeats, incidents) | `.page-head` (caption + h1) → `.panel` with `.table`. Empty: `.empty` in the panel. Primary action in the toolbar. |
| Detail (monitor, incident) | `.page-head` → `.cols`: facts panel (`.kv`) + activity panel (`.timeline` or checks table) |
| Form (create, edit) | `.panel` > `.panel__head` + `.panel__body` > `.form` |
| Settings | `.tabs` (General, API keys, Alerts, Allowlist) → one panel per tab |
| Kanban | `.page-head` with `.tabs` → `.board` |
| Status page | h1 → `.headline` → per item: title row + `.uptime` → incidents as `.timeline` → footer links |

## 3.4 Copy rules

- Name things by what a TL knows: "Ping URL", not "heartbeat token endpoint".
- Buttons say what happens: "Save heartbeat", "Revoke key", "Post update".
- Errors say what to do: "Enter a name, for example nightly-settlement."
- Times: relative in lists ("6m ago"), absolute with zone on detail pages ("16 Oct, 14:02 WIB").
- Durations: `6m 18s`, `2h 4m`, `3d 2h`. Latency: `182ms`. Uptime: two decimals, `99.91%`.

## 3.5 Do and don't

| Do | Don't |
| --- | --- |
| Put color in icons and dots | Color a whole row or section |
| Use one `--primary` button per view | Put two light pills side by side |
| Separate groups with hairlines | Add shadows to flat panels |
| Use `.cap` for IDs, times and counts | Uppercase headings or use weight 700 |
| Fade colors on hover in 160 ms | Pulse, bounce or animate on scroll |
| Write tokens on `:root` | Write a hex value in a template or a component rule |
