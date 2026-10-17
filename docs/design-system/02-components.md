# 2. Components

Templates call the macros in `templates/ui/macros.html`. They never repeat the markup by hand. A macro is added to `macros.html` when the first story needs it: add the macro, then use it. Today it holds `icon`, `st`, `state`, `chip`, `notice`, `secret`, `empty`, `input`. A new component gets a class in `app.css`, a macro, and a row here.

| Component | Class | Macro | Use |
| --- | --- | --- | --- |
| Button | `.btn` + `--primary`, `--accent`, `--danger`, `--ghost`, `--sm`, `--lg` | `btn(label, kind, href)` | One `--primary` per view. `--primary` and `--accent` are the same magenta brand button, as on the landing page. `--danger` for revoke and delete. No icon-only buttons without `aria-label`. |
| Status icon | `.st` + `--up`, `--down`, `--issue`, `--unknown`, `--triage`, `--backlog`, `--progress`, `--done` | `st(kind)` | 13 px ring or disc |
| State | `.state` + `--up`, `--down`, `--issue`, `--unknown` | `state(kind, label)` | Tone pill: tinted fill and coloured word, no dot. Monitor, heartbeat and incident state. |
| Chip | `.chip` + `--critical`, `--warning`, `--investigating`, `--identified`, `--monitoring`, `--resolved` | `chip(kind, label)` | Severity and status-update labels, as tone pills like `.state`. Neutral facts use `.tag`. |
| Panel | `.panel`, `.panel__head`, `.panel__body`, `.panel--float` | `panel_start(title)` / `panel_end()` | The one boxed surface. Group content, do not box every block. |
| Table | `.table-wrap` > `.table`, `.table__title`, `.table__sub`, `.t-right` | none (plain HTML) | Lists of monitors, heartbeats, incidents, keys, users. Rows 56 px, 14 px text. The `.table__title` link covers its row, so any cell opens the item; other links and buttons in the row stay on top. `.table--rich` rows add `.table__tags`. |
| Field | `.field`, `.label`, `.input`, `.select`, `.textarea`, `.help`, `.error`, `.field--error` | `input(name, label, value, error, help)`, `select(...)`, `textarea(...)` | Every form control has a `<label for>`. The error replaces nothing: it adds a line under the control. |
| Check and radio | `.check`, `.choices` | `check(name, label, checked)` | Radio groups sit in a `fieldset` with a `legend.label`. |
| Form | `.form`, `.field__row`, `.form__actions` | none | Max 560 px wide, or the full `.panel--form` (720 px). Actions at the bottom, primary first. |
| Form page | `.form-page` > `.panel.panel--form` + `.form-aside` > `.steps` | none | Create and edit pages. The aside says, in numbered sentences, what the thing will do. It sits beside the form on wide screens and below it on phones. |
| Choice cards | `.choices--cards`, `.choices--grid` | none | Severity radios as cards; the checked card takes the brand tint. Long checkbox lists in a grid. |
| Overall line | `.overall` + `--down`, `--issue`, `--unknown` | none | One sentence above a list that sums up its state: "1 of 4 monitors is down." |
| Sparkline | `svg.spark` > `.spark__line`, `.spark__fail` | none | The monitors list: 24 hourly mean response times from `checks::overview`, with failed hours shaded red. |
| Deadline bar | `svg.due` > `.due__track`, `.due__fill`, `.due--late` | none | The heartbeats list: the share of the wait before the deadline that has passed. |
| Top bar and tabs | `.appbar-band` > `.appbar` (`.appbar__brand`, `.appbar__project`, `.appbar__end`, `.appbar__link`) and `.tabbar` > `a[aria-current="page"]`, `.tabbar__count`, `.tabbar__end` | none | One sticky band. The top bar's area links (Projects, Admin) show the current area as a raised pill. The tab bar shows a project's areas with a count beside Monitors, Heartbeats, open Incidents and open Tickets; the open tab takes an inset brand underline. It scrolls sideways on a phone. |
| Account menu | `button.user` (`popovertarget`, `aria-label="Account: …"`) > `.user__avatar`; `.menu[popover]` > `.menu__who`, `.menu__item`, `.menu__item--danger`, `.menu__rule` | none | The avatar at the top bar's right end. The menu holds All projects, the theme switch and Sign out, so signing out takes two clicks. The Admin menu uses the same `.menu`. Esc and a click outside close both. |
| Side navigation | `.content--side` > `.sidenav` > `a[aria-current="page"]`, `.content__main` | none | Only an area with sub-pages: a project's Settings (General, Alerts, API keys, Allowlist) and Admin (Users, System). Each link has an icon; the current one is a raised card with a 2 px brand bar on its left. A row of links on a phone. |
| Panels | `.panels` > `.panel`, `.panel--wide` | none | Two panels side by side from 1180 px; a wide panel takes the row. Settings, alerts and System. |
| Tally | `.tally` > `.tally__cell` + `--wait`, `--bad`, `.tally__n` | none | Three counts side by side, such as emails sent, waiting and failed. |
| Range | `svg.range` > `.range__track`, `.range__fill`, `.range__default` | none | Where a limit sits between 1 and its maximum on a log scale, with a mark at the default; or a cap's use. |
| Limit row | `.limits` > `.limits__group` > `.limit` > `.limit__text`, `.limit__set` | none | One editable limit: what it does, its input, its range and its default. |
| Cap card | `.caps` > `.capcard`, `.capcard__n` | none | A fixed cap, with its use when Monita counts it. |
| Page head | `.page-head` > `h1` + `.page-actions` | none | One row: the title, then the page's actions at the right end, such as `.page-search` on Projects. A detail page uses `.hero` with `.page-actions` instead. |
| Breadcrumbs | `nav.crumbs` | none | Detail and form pages only, above the title: "Monitors / API". Hidden when empty. |
| Table bar | `.tablebar` > `.tablebar__count`, `.tablebar__tools` (`.tablebar__search`, `.select--sm`) | `option(value, label, current)` | The band on top of a table panel: the count on the left ("2 of 4 monitors"), then search and filters as a `GET` form. A select applies at once (`data-autosubmit`); Enter applies the search. |
| Project card | `.pcards` > `a.pcard` (`.pcard__top`, `.pcard__badge`, `.pcard__name`, `.pcard__desc`, `.pcard__foot`) | none | The projects home: one card per project, its description in two lines at most, then a footer band of facts. New project is the page action only. |
| Project badge | `.switcher__badge`, `.badge--icon` | `badge(slug, icon_at, letter, class)` | The project's icon, or its first letter when it has none. In the top bar, the cards, the overview and General. |
| Icon picker | `.iconpick` > `.iconpick__preview`, `input[type=file][data-icon-file]`, `input[name=icon][data-icon-value]` | `projects/icon_field.html` | The file input has no name. `app.js` cover-crops the picture to 128 px, writes WebP (PNG where WebP is missing) under 32 KB as base64 into the hidden field, and shows a preview. Without the script no icon is sent. |
| Owner picker | `input[type=email][list=owners]` + `datalist#owners` | `projects/owner_field.html` | A super admin types to search the eligible owners by email. The browser lists the matches; the server checks the email again. |
| Tile | `.tiles` > `a.tile` > `.tile__icon`, `.tile__body`, `.tile__text`, `.tile__go` | none | The project Overview: one tile per area with its state ring and one sentence. |
| About column | `aside.about` > `h2`, `ul.facts`, `ul.facts--stats` | none | The project Overview's right column: icon facts, then counts. |
| Uptime bar | `.uptime`, `.uptime__bars` > `span.is-partial / .is-down / .is-none`, `.uptime__legend` | `uptime(days)` | 90 spans, oldest first. `title` gives date and uptime. |
| Timeline | `.timeline`, `.timeline__item`, `.timeline__mark`, `.timeline__meta`, `.timeline__body`, `.comment` | `timeline_item(update)` | Status updates, newest first. TL updates in `.comment`, Monita's own entries as plain text. |
| Board | `.board`, `.board__col`, `.board__head`, `.board__count`, `.card`, `.card__title`, `.card__meta` | `ticket_card(ticket)` | The ticket board. The column head carries the stage ring. |
| Status headline | `.headline` | `headline(state, text)` | Top of the public status page |
| Notice | `.notice` + `--ok`, `--error` | `notice(kind, text)` | Result of the last action, or a page-level error |
| Secret | `.secret` > `code` + copy button | `secret(value)` | API key shown once, ping URL |
| Key-value | `.kv` (`dl`) | none | Incident facts, monitor settings |
| Empty state | `.empty` | `empty(title, text, action)` | Every list with no rows says what the thing is and how to add the first one. |
| Dialog | `dialog.dialog` > `.dialog__head`, `.dialog__body`, `.dialog__foot` | `dialog_head(id, title)` | Native `<dialog>`. A button with `data-dialog="id"` opens it; `data-close` closes it; `data-open` opens it on load when its form came back with an error. Quick creates: New project, Invite tech lead, New API key, Post update, Reassign. |
| Confirm | `button[data-confirm][data-confirm-text]` and the shared `#confirm` dialog in the app layout | none | Risky actions ask first: delete a monitor or heartbeat, revoke a key or ref ID, disable a user. The dialog's button repeats the action's label. Without the script the form submits at once. |
| Caption | `.cap` | none | 12 px muted sentence case. Never an uppercase label above a title. |
| Quiet link | `.link-quiet` | none | "Known issues →". Arrows live here, never inside buttons. |
| Detail hero | `.hero`, `.orb` + `--down`, `--issue`, `--unknown`, `.hero__state`, `.tag` | none | Top of a monitor or heartbeat page: live state, name, cadence |
| Stat | `.stats` > `.stat` + `--up`, `--down` | `checks/insight.html` | Four numbers under the hero |
| Chart | `.chart-panel`, `svg.chart`, `.seg`, `.chart-legend` | `checks/insight.html` | Server-drawn SVG from `checks::chart`. No script. |
| Availability | `.avail__cell`, `.avail__bar` + `.avail--good`, `--fair`, `--poor` | `checks/availability.html` | Period table under the chart |

## 2.1 Form states

| State | Style |
| --- | --- |
| Validation failure | Server returns 422 with the whole page, fields marked `.field--error`. |
