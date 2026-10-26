# 3. Patterns

## 3.1 Shells

| Shell | File | Structure |
| --- | --- | --- |
| Signed in | `layouts/AppLayout.tsx` (inside `SignedInLayout`) | `TopBar` then `PageFrame`. Projects, Incidents, Tickets and Admin pages. `SignedInLayout` loads the session once, so moving between areas keeps the header. |
| Project | `layouts/ProjectLayout.tsx` | `TopBar` (project crumb, `PageTabs`) then `PageFrame` then the page, then the primitive's footer illustration. The header stays up while the project loads. |
| Admin | `layouts/AdminLayout.tsx` | `SidebarFrame` with `SideNav` (User management, Rate limits, Outgoing email, Telegram, Time zone, System information). Someone who is not a super admin or supervisor gets the not-found page. |
| Settings | `layouts/SettingsLayout.tsx` | `SidebarFrame` with `SideNav` (General, Status page, Alerts, API keys, Secrets, Allowlist) |
| User settings | `layouts/AccountLayout.tsx` | `SidebarFrame` with `SideNav` (Profile, Password, Two-factor sign-in), for the signed-in user's own account. The avatar menu's **User settings** opens it. |
| Public | `layouts/PublicLayout.tsx` | Sticky header (project name, homepage link, `ThemeToggle`, centred `PageTabs`), `PageFrame`, footer |
| Bare | `layouts/BareLayout.tsx` | Sign-in and invite: the brand, one card (max 420 px), a footer, `ThemeToggle`, and an optional scene along the bottom |

Under 768 px (`md`) a side navigation becomes one scrolling row. The tab bar scrolls sideways. Area links in the top bar show icons only below 1024 px.

## 3.2 Navigation

| Place | Items |
| --- | --- |
| Top bar | Mark and project crumb (outside a project: a way back to the last one opened, from `lib/lastProject.ts`, kept per signed-in email and forgotten when it answers 404), then the area links Projects, Incidents, Tickets, Admin (super admin or supervisor), then the account menu |
| Account menu | Who is signed in and their role, Documentation, User settings, Toggle theme, Sign out. On a phone Admin moves in here. |
| Project tabs | Overview, Monitors, Heartbeats, Incidents, Maintenance, Tickets, Changelog, Settings. A Status page button sits at the right end and leaves the app for `/status/{slug}`. |
| Settings side nav | General, Status page, Alerts, API keys, Secrets, Allowlist |
| Admin side nav | User management, Rate limits, Outgoing email, Telegram, Time zone, System information |
| Public tabs | Uptime (also Past uptime), Incidents (also History), Issues, Ticket, Changelog. A tab the project turned off is gone, and its URL answers 404. |

Area links in the top bar:

- Projects, Incidents and Tickets show across the signed-in user's projects. A tech lead sees only their own projects.
- The open area is a raised pill.
- Admin opens the admin page opened last (`lib/adminPage.ts`, kept per browser). The first time it opens User management.
- Documentation sits in the account menu and leaves the app in a new tab.

Only Settings and Admin have a side navigation. Every list stays full width.

Numbers on tabs flag a problem. They are never a total.

| Tab | Number | Color |
| --- | --- | --- |
| Monitors | Monitors that are down | Red |
| Heartbeats | Heartbeats that are down | Red |
| Incidents | Open incidents | Red |
| Maintenance | Monitors and heartbeats paused or in an active window now, each once | Neutral |
| Tickets | Tickets in Triage | Neutral |
| Overview, Changelog, Settings, the top bar, the public tabs | None | |

A tab shows no number at zero. A total (all monitors, all releases) never appears on a tab.

## 3.3 Page recipes

All pages open with `PageHeader`. A detail page puts `Crumbs` above it. A form opens in a sheet over its page, so it has no crumbs of its own.

| Page | Recipe |
| --- | --- |
| Projects | `PageHeader` (`ViewToggle`, `PageSearch`, `RefreshButton`, New project) then the project count with Clear filters and the Owner (when the list has owners to pick) and Sort `FilterMenu`s, then a grid of 20 `ProjectCard`s and a `Pager`. The list view is a `TableCard` with the same `FilterMenu`s and `SortHead` on Project. The view is remembered. The last `?q=&owner=&sort=&page=` reopens on a bare visit. The page reads again every 30 seconds. New project opens `NewProjectSheet`. |
| Incidents (all projects) | `PageHeader` (`RefreshButton`) then a `TableCard` of open incidents, critical first, with `ListFilters` (search by title, `INC-n`, `#n` or project name; state with Open, Resolved and Open and resolved; severity), `SortHead`s and a `Pager`. The query lives in the URL and is remembered. An incident opened from here has an All incidents crumb back. |
| Tickets (all projects) | `PageHeader` (`RefreshButton`) then a `TableCard` of tickets in Triage with search, a date `SortHead` and a `Pager`. A ticket opens in the `TicketPanel` over the list. |
| Overview | `PageHeader` (`ProjectBadge` lead, Edit project) then two columns. At left: a Last 24 hours card with the monitor and heartbeat summary sentences, a Refresh button and `OverviewChart` (`app/OverviewChart.tsx`). The server lays the chart out in a 1000-wide view box. It holds up to 5 response-time lines on a log scale in `--series-1` to `--series-5` (never red, green or yellow), the other monitors as faint lines, red shading for down time, and up to 6 heartbeat lanes of ping dots plus a "N more" lane. A crosshair readout lists every line and lane in the 15-minute slot. Lines, faint lines, lane names and legend entries open their page. Below 640 px the chart scrolls sideways inside the card. Then an Open incidents `TableCard` of `LinkRow`s with an All incidents link. At right, an 18 rem sticky About `aside`: the description, homepage link, owner (with the owner's picture, or the initial), created date, the Triage ticket count, API keys, then links to Status page tabs, an alert line per level ("Critical: 2 cards.") and the allowlist. Edit project is the one header action. It turns About into the description, homepage and icon form, and the header shows Discard changes and Save. |
| List (monitors, heartbeats, incidents) | `PageHeader` (title, `RefreshButton` on monitors and heartbeats, primary action) then `Overall` (monitors, heartbeats), then `TableCard` (count, Clear filters and `ListFilters`) holding a `Table` with `SortHead`s and `LinkRow`s. Monitor rows carry a `Sparkline` and the last check. Heartbeat rows carry a `DueBar`, the next ping due and the newest ping, good or failed, with a fail's cause in red. Every wait or interval reads in words ("1 minute"). Monitors and heartbeats read again every 30 seconds. For a writer, the first column is a `Checkbox` per row, with a heading box that picks the rows shown. While rows are picked, a `BatchBar` replaces the count in the band: "N selected", Pause (through `ConfirmDialog`, when a picked row runs; it pauses only those), Resume (when a picked row is paused; it resumes only those; both read "Pause N" and "Resume N" for a mixed pick), Plan maintenance, Delete and Clear selection. The last column is a `SourceRowMenu`: Edit, Pause or Resume, Plan maintenance, Delete. Delete opens `DeleteSourcesDialog`. A paused row has grey stripes and a row in maintenance has yellow stripes (`rowTape`, 3.11). Each row leads with a `PinButton`. A supervisor sees no boxes and no menu. |
| Detail (monitor) | `Crumbs`, `PageHeader` (title, `meta`: state, URL, severity, "On the status page"; actions: `RefreshButton`, Details, Edit), a `StateStamp` while the monitor is paused or in maintenance (3.11), `StatCards`, `ChartCard`, `AvailabilityCard` at full width, then a checks `TableCard` with `HistoryPager`. Details is a half sheet (`DetailsSheet`) with the request (URL, method, header names, body size, timeout), a curl command that repeats the check, the pass rule, the failure rule and the settings. |
| Detail (heartbeat) | Same shape, with the same `StateStamp`. `meta` holds the state, "Every X, can be late by Y" and severity. Stats are Up or Down for, Next ping due, Last ping and Incidents. A failed last ping turns Last ping red. Details lists the Ping URL, "Call it from your job" (a Shell, Node.js or Python `ToggleGroup`, remembered per browser, over `CodeCopy` blocks for success, failure with a reason and a starter script) and the settings. Until the first ping, a card above the stats holds the URL and a How to call it button that opens the same panel. The pings table adds "What the job reported". |
| Monitor and heartbeat form | `FormSheet` over the page, half width, deep-linked at `/monitors/new`, `/monitors/{id}/edit` and the heartbeat paths. The header has the title and a `GuideSheet` button. The body is a `FieldGroup`. Right under the name sits `PublicField`: the legend "Who sees it" over two `ChoiceCards`, "Only your team" (the default) and "Also the status page". Monitor sections after it: Request (URL, method, body), Authentication (a bearer token, user name and password, or a key in a header, each from a project secret), Headers (name and value rows), "A check passes when" (status code, response text, self-signed certificate), then Alerts (`SeverityField`, failure count 1 to 99, `WhenItFails`, the certificate warning). Heartbeat fields: name, "Who sees it", "Runs every", "Can be late by", then `SeverityField` and `WhenItFails`. A hint goes in the placeholder, not under the field. The footer has Delete (edit only) at the left, Cancel and Save at the right. |
| Detail (incident) | `Crumbs`, `PageHeader` (state, severity, INC number and origin; action: Post update), then two columns. At left, "Updates, newest first": a thread of comment cards, each with a round initial, the `LabelBadge`, the author (Monita for an automatic update) and the time. At right, an 18 rem sticky sidebar of `Side` blocks: Affects, Cause, Dates, How it closes. Reclassify, beside Post update for a writer, is a menu (Planned work, False alarm, Count it again) that opens a `ConfirmDialog` explaining the change. A reclassified incident shows a `ClassNotice` above the thread (3.11). Post update is a half `FormSheet`. It starts from the first label with that label's template. A new label swaps the message only when the box is empty or still holds an untouched template. Labels are `ChoiceCards`. A label the incident cannot take shows dashed with the reason. |
| Report incident | `FormSheet`: title, `SeverityField`, a message, and the affected items. Items come from one search over two lists, Monitors and Heartbeats, each with a count ("5 of 25 monitors") and its own scroll. Down and Issue items show a badge. A "Chosen:" line keeps picked items in view while a search hides them. Names sort with numbers in order. |
| Ticket board | `PageHeader` (Issue ref ID, Connect your app, New ticket) then `TicketBoard` with Triage, Backlog, In progress and Done, each with a one-line note. A column shows 25 cards and a Show more. Done links to `/tickets/done`, a paged table. A card drags to another column. A drop on Done opens a small dialog for the close reason. |
| Ticket panel | A wide `Sheet` over the board at `/tickets/{number}`. See `TicketPanel` in 2.8. The Column select saves at once. |
| Changelog (team) | `PageHeader` (New release) then a `TableCard` of releases with search, a date `SortHead` and a `Pager`, 25 a page. A release opens in a `FormSheet`: version, release day, title, and Added, Changed, Fixed and Removed lists with one input per change (Enter adds the next). Every bad field shows at once. Delete goes through `ConfirmDialog`. |
| Maintenance | `PageHeader` (`MaintenanceGuide`, Plan maintenance) then a `TableCard` of windows with search, a State filter (Scheduled, Active, Ended; `ListFilters severity={false}`), a When `SortHead` that sorts by start on the server, and a `Pager`, 20 a page. Each row names its items as links to their pages, with the status-page message under them. Scheduled and active windows offer Edit, which opens `WindowForm` on the window at `/maintenance/{id}/edit` (Q50). An active window's form disables Starts and shows its items as text. Scheduled windows offer Delete, active ones End now, both through `ConfirmDialog`. A supervisor sees no buttons. Below it, `PausedNow` is a `TableCard` headed "N paused" ("Paused now" while it loads). Its rows have grey stripes, and a Resume button for a writer. It shows "Nothing is paused" when empty. Plan maintenance opens `WindowForm` in a `FormSheet`; `?item=` picks sources from a row menu, and the list stays out of that URL (`useServerList(key, active)`). |
| Settings page | `SidebarFrame`, then `SettingsPage` (title, actions, optional `toc`) holding `SettingsSection`s. See 3.9. |
| Admin Users | `PageHeader` (Invite new user) then a `TableCard` with search, role and status filters. Invite is a `FormSheet` with the role as two `ChoiceCards`. Disable goes through `ConfirmDialog`. |
| Admin pages | One topic per page. Rate limits: `SettingsSection`s by group (Monitoring, Visitors and apps, Bug reports), each limit a row with a plain label, one line of help and a small number input, then Discard changes and Save; a Built-in maximums table below. Outgoing email: a Send a test email section (off until a mail server is set) and an Email delivery section with a badge. Time zone: the offset. System information: server facts in plain words. |
| Public Uptime | `PageHeader`, a headline sentence with a tone border and "N of M systems up", then Systems: a two-column grid of `SystemCard`s with View past uptime. Systems with no bars sit under "N more systems". The tab reads again every 60 seconds and has no Refresh button. |
| Public Past uptime | `PageHeader` with a link back to the current status, then one card: a system `Select` and `WindowNav`, then three `MonthGrid`s. A system that is not public answers 404. |
| Public Incidents | `PageHeader`, Happening now (an `IncidentList`, only while an incident is open), then Past incidents with `WindowNav` and one `IncidentMonthList` per month. Every list uses `IncidentRow`. History shows the same rows under month headings. |
| Public Issues | `PageHeader`, then `TicketBoard` with Backlog, In progress and Done. A card shows the title, the number and the age. Never the description or the reporter. |
| Public Ticket | One tab for reporting and following a bug, in two columns. At left, a Report a bug `Card` form with a required ref ID, a title, a description and the page URL. At right, a Check your tickets card that asks for the ref ID and lists that reporter's tickets (`MyTickets`) below it. |
| Public Changelog | `PageHeader`, then the version list (`VersionIndex`) at left and one `ReleaseCard` at right, newest by default, with Newer and Older links. |
| Sign-in, invite | `BareLayout` and one `Card`. Sign-in also has the playground. See 3.8. |

## 3.4 Forms

- Build with `FieldGroup`, `Field`, `FieldLabel`, `FieldError`. Wrap a related set in `FieldSet` and `FieldLegend`.
- Put `data-invalid` on `Field` and `aria-invalid` on the control when a field has an error.
- A required field uses `<FieldLabel required>` and `aria-required` on the control. There are no "(optional)" labels. An unmarked field is optional.
- A needed hint goes in the placeholder or in the label text. A field has no subtitle and no `FieldDescription`.
- A refused form returns 422 with `error.fields`. The page reads `ApiError.fields[name]` (`lib/api.ts`) and shows every bad field at once in `FieldError` under that field. A message with no field shows in an `Alert`, a line of red text or a toast.
- An option set of 2 to 7 uses `ToggleGroup` or `ChoiceCards`. Options that turn on and off alone use `CheckCards`. A longer set uses `Select`. Never mark a choice with the brand color.
- A form with several fields opens in a `FormSheet` (3.1 and 2.5), not in a dialog. It keeps one primary `Button` in the footer. A busy button is disabled and shows a `Spinner`.
- Forms use `noValidate`. The server validates.

## 3.5 States

| Case | Component | Text |
| --- | --- | --- |
| Loading | `Skeleton`, the size of what it replaces | None |
| Nothing yet | `TableEmpty` with `title` and at most one sentence in `description` | "Add your first monitor." plus one sentence on what a monitor does |
| A filter hides every row | `TableEmpty filtered noun="monitors" onClear` | "No monitors match these filters." and a Clear filters button |
| Risky action | `ConfirmDialog` | Title asks the question. Description says what is lost. The button repeats the action. |
| Result of an action | `toast.success` or `toast.error` | One sentence |
| A secret shown once | `Alert` with `CopyField` | "Key created. Copy it now: Monita shows it only once." |
| A supervisor | The page, with no create, edit or delete control | See 3.10 |

`TableEmpty` sits inside the list's `TableCard` or `Card`. It replaces the table.

## 3.6 Copy rules

- Write whole sentences. Do not stack fragments such as "No X, no Y" or "One X, one Y".
- State what is true and literal. Use no metaphors.
- Say "tech lead", not "TL". Say "ticket board", not "kanban".
- Name things by what a tech lead knows: "Ping URL", not "heartbeat token endpoint". Say "Can be late by", not "grace time". Say "No data yet", not "Waiting".
- A button says what happens: "Save heartbeat", "Revoke key", "Post update".
- An error says what to do: "Enter a name, for example nightly-settlement."
- Use no eyebrow label, no uppercase label and no dot separator between facts. Separate facts with spacing or a comma.
- Times in lists are relative ("6m ago"). Times on detail pages are absolute UTC. Intervals and waits read in words: "30 seconds", "1 minute".
- Durations: `6m 18s`, `2h 4m`, `3d 2h`. Latency: `182 ms`. Uptime: two decimals on the status page (`99.91%`).

## 3.7 Do and don't

| Do | Don't |
| --- | --- |
| Put state color in a `Badge` tone pill | Put a decorative dot or a dot before a label |
| Put a list's count in its `TableCard` band | Put an uppercase label above a title |
| Use the brand color for the mark, the primary button, focus, links, the chart line and Moni | Use the brand color to mark a state or a chosen option |
| Use green for Done and a Fixed reason, grey for Triage, Duplicate and Won't fix | Use the brand color for a ticket stage |
| Put a number on a tab only to flag a problem | Put a total on a tab |
| Import icons from `components/icons` | Import an icon pack directly |
| Use tokens (`bg-primary`, `text-down`) | Write a hex value in a component |
| Use one `default` button per view | Put two brand buttons side by side |
| Separate groups with hairlines | Add a shadow to a flat card |
| Draw charts as SVG from `checks::chart` | Add a chart library |
| Keep muted text readable in both themes | Use grey text that fades into the background |
| Page every list the server reads | Render every row of a list that can grow |
| Wrap create, edit and delete controls in `Writable` | Show a supervisor a control that the server will refuse |

## 3.8 Footer illustrations and the playground

Moni's drawing rules are in 1.11. This section says where illustrations go.

The footer sits under every page of a primitive's tab: Monitors, Heartbeats, Incidents, Tickets and Changelog, including detail pages. Overview and Settings have none. It is a full-width band at the foot of the project shell (`mt-auto`), with a hairline above, the `--paper` surface, a height of 200 px (150 px on a phone) and a ground line near the bottom.

Each footer has three parts.

| Part | Rule |
| --- | --- |
| Scene | A 560 by 200 SVG in the middle. A `Window` stands on the ground at the left. Moni stands at the right, faces left and holds or points at something. |
| Edges | On each side, a dot grid and a tile of two or three outline glyphs for the primitive in `currentColor` at low opacity. Each edge fades toward the scene and toward the top, so the texture never meets a hard line. |
| Flow | Dashed lines with an arrowhead show what moves. Brand marks what Monita sends. Green marks the answer. |

| Primitive | What the scene shows |
| --- | --- |
| Monitors | Moni checks a website through a magnifying glass, on a timer, and the answer is 200 OK. |
| Heartbeats | A scheduled job runs, then calls Monita. Moni catches each beat. |
| Incidents | The team posts each step to the status page. Moni announces it, and the last step is Resolved. |
| Tickets | A board of four columns. Moni carries a ticket over to Done. |
| Changelog | A release goes out. Moni waves a flag over the notes. |

Sign-in playground: seven Monis play along the bottom of the sign-in page (a kite with the mark, a game of catch, a jump, a tower of uptime blocks in green, bubbles), and one peeks over the top edge of the card. The scene fills the height the page leaves. It shrinks first when the screen is short, and it hides below 680 px of height. A phone crops its sides so the Monis stay large. The page fits the screen at every size without a scroll. Only a screen too short for the card itself scrolls.

## 3.9 Settings pages

A settings page is a `SettingsPage` of `SettingsSection`s in one column. Each section is a card: a title and one sentence, an optional action at the right of the header, the fields, then a `--paper` footer. The footer holds a note at the left (when the project was created, the status page URL) and its actions at the right.

| Page | Sections |
| --- | --- |
| General | Project (name and slug read-only, homepage, description, icon), Owner, and a Danger zone that only a super admin sees. Project edits in place with Save. The danger zone deletes the project after the name is typed. |
| Status page | Public tabs: `CheckCards` for Uptime (always on), Incidents, Issues, Ticket and Changelog, with the status page URL in the footer. Custom status page: a `reference` section. It offers a downloadable one-file HTML page, then a `Tabs` row with one tab per public endpoint. Each tab shows who can call it, a `CodeCopy` request, an example response (`lang="js"`) and its rules. |
| Alerts | Two `SettingsSection`s. Alert cards (action: Add alert card) and Ticket alerts (action: Add ticket card). Each holds a two-column list of card tiles. A tile shows the name, the kind unless the name says it, grey badges for what it hears, where it sends, an `AlertSwitch`, Send a test and Edit (not on the owner's cards). An incident card shows grey `SeverityBadge`s for its levels. A ticket card shows event badges: Daily digest, New tickets, Ticket moves. A level no card hears gets a one-sentence `Alert` above the incident list. Add and Edit open `AlertCardSheet`, a `FormSheet`: `DestinationFields` (a Global Telegram or Custom switch when a bot is set up, then Sends by and the kind's fields) and Listens to. The form has no Name field. The server names the card after its kind. Delete sits at the footer's left. On this page lists the two sections. The page action opens `AlertsGuide`, a half sheet with how alerts work, the payload samples and delivery. |
| API keys | One `TableCard` with New API key in a `FormSheet`. A new key shows once in an `Alert` with `CopyField`. |
| Secrets | One `TableCard` of secrets by name with what uses each, New secret and Replace value in a `FormSheet`, and Delete, which is off while a monitor uses the secret. A value is never shown after it is saved. |
| Allowlist | One section with the allowed IP ranges. |

On this page (`toc`) appears only on a page with several long sections, from `xl`.

## 3.10 Read-only pages for a supervisor

A supervisor sees every project and the admin pages and changes nothing.

- The server answers 403 to any method but GET and HEAD. The UI hides what the server would refuse.
- Create, edit and delete controls sit inside `Writable`, or inside a component that guards this on its own (`FormSheet`, `ConfirmDialog`, `SettingsSection`).
- A `SettingsSection` shows its fields disabled, with its value, and drops its footer and header action. A `reference` section stays fully usable.
- No heartbeat ping URL, no header values and no request body reach a supervisor, so their Details panels and forms hide those parts.
- The account menu shows the role Supervisor.

## 3.11 Paused, maintenance and held

Barrier tape marks a source that Monita is not watching as usual. The tape colour and the word say which case it is. Tape is a pattern, not a state badge: the badge in the State column still carries the state.

| Case | Tape | Word | Where |
| --- | --- | --- | --- |
| Paused | Grey, `--muted-foreground` | "Paused" | The `StateStamp` on the page, and a striped row in the monitor and heartbeat lists and in `PausedNow`. |
| In maintenance | Yellow, `--issue` | "In maintenance" | The `StateStamp` and a striped row. |
| Held failures | Grey stripes in the chart, `--neutral-strong` at 25% | The hover title "Maintenance: {title}, {duration}" | `ChartSvg`, over the window's span. |
| Reclassified incident | Grey, `--neutral-strong` | "Planned work" or "False alarm" | `ClassNotice` above the thread. Lists show the same words in an outline `Badge` under the title. |

- `tape(token, strength)` in `app/StateStamp.tsx` returns the stripes: a repeating 45 degree gradient, 10 px of colour and 10 px of gap, mixed from the token. `rowTape(state, covered)` picks the token for a list row: yellow when a window covers it, even when an open incident keeps the state Down (strength 10 for paused, 14 for maintenance). Dark mode multiplies the strength: `--tape-boost` (3.4) for the stamp's bars and `--row-tape-boost` (1.8) for a row, whose text sits on the stripes. A tint that reads on white turns murky on the dark page.
- `StateStamp` sits under the page title as a bordered card with `role="status"`. It has a stripe bar of 10 px at the top and at the bottom, the word in a bold, uppercase, spaced box with a 2 px border in the tape colour, and one sentence of what it means. A maintenance stamp adds a "See the window" link to the Maintenance tab and says when the window ends.
- Yellow is also the colour of Issue. The word tells them apart: Issue is a state badge, and maintenance is always "In maintenance" or "Maintenance" on tape.
- Rows keep their text colours. The stripes are a background only, so the text keeps its contrast.
- The Maintenance tab carries the same tape on its `PausedNow` rows. Its number is neutral, since planned work is not a fault.
