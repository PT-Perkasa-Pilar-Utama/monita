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
| Panels | `.panels` > `.panel`, `.panel--wide` | none | Two panels side by side from 1180 px; a wide panel takes the row. Settings and alerts. |
| Tabs | `.tabs` > `a[aria-current="page"]` | `tabs(items, current)` | Project settings sections, ticket views |
| Uptime bar | `.uptime`, `.uptime__bars` > `span.is-partial / .is-down / .is-none`, `.uptime__legend` | `uptime(days)` | 90 spans, oldest first. `title` gives date and uptime. |
| Timeline | `.timeline`, `.timeline__item`, `.timeline__mark`, `.timeline__meta`, `.timeline__body`, `.comment` | `timeline_item(update)` | Status updates, newest first. TL updates in `.comment`, Monita's own entries as plain text. |
| Board | `.board`, `.board__col`, `.board__head`, `.board__count`, `.card`, `.card__title`, `.card__meta` | `ticket_card(ticket)` | Kanban. The column head carries the stage ring. |
| Status headline | `.headline` | `headline(state, text)` | Top of the public status page |
| Notice | `.notice` + `--ok`, `--error` | `notice(kind, text)` | Result of the last action, or a page-level error |
| Secret | `.secret` > `code` + copy button | `secret(value)` | API key shown once, ping URL |
| Key-value | `.kv` (`dl`) | none | Incident facts, monitor settings |
| Empty state | `.empty` | `empty(title, text, action)` | Every list with no rows says what the thing is and how to add the first one. |
| Dialog | `dialog.dialog`, `.dialog__body`, `.dialog__foot` | `confirm(id, title, text, action)` | Confirm revoke and delete. Native `<dialog>`, no JS library. |
| Caption | `.cap` | none | 12 px muted sentence case. Never an uppercase label above a title. |
| Title count | `.page-title` > `h1` + `.count` | none | A number beside a page title: "Monitors 4" |
| Quiet link | `.link-quiet` | none | "Known issues →". Arrows live here, never inside buttons. |
| Detail hero | `.hero`, `.orb` + `--down`, `--issue`, `--unknown`, `.hero__state`, `.tag` | none | Top of a monitor or heartbeat page: live state, name, cadence |
| Stat | `.stats` > `.stat` + `--up`, `--down` | `checks/insight.html` | Four numbers under the hero |
| Chart | `.chart-panel`, `svg.chart`, `.seg`, `.chart-legend` | `checks/insight.html` | Server-drawn SVG from `checks::chart`. No script. |
| Availability | `.avail__cell`, `.avail__bar` + `.avail--good`, `--fair`, `--poor` | `checks/availability.html` | Period table under the chart |

## 2.1 Form states

| State | Style |
| --- | --- |
| Validation failure | Server returns 422 with the whole page, fields marked `.field--error`. |
