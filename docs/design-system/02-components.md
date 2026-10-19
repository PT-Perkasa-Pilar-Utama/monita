# 2. Components

Order of choice: a shadcn component in `web/src/components/ui/`, then a Monita component below, then new markup. A new Monita component goes in `components/app/` and gets a row here.

## 2.1 shadcn components, as Monita restyles them

| Component | File | Variants and rules |
| --- | --- | --- |
| Button | `ui/button.tsx` | `variant`: `default` (brand magenta), `secondary`, `outline`, `ghost`, `destructive`, `danger`, `link`. `size`: `default` (36 px), `sm` (32 px), `lg` (44 px), `icon`, `icon-sm`. Flat, with the input's 8 px corner (`rounded-lg`) and a 150 ms color fade. |
| Badge | `ui/badge.tsx` | `variant`: `up`, `down`, `issue`, `unknown`, `neutral`, `destructive`, `outline`, `secondary`, `default`. A tone pill: tinted fill, colored word, no dot, 6 px corner. |
| Label | `ui/label.tsx` | `required` adds a red asterisk. The control also gets `aria-required`. |
| Field | `ui/field.tsx` | `FieldGroup`, `Field`, `FieldLabel`, `FieldError`, `FieldSet`, `FieldLegend`, `FieldSeparator`. `Field orientation`: `vertical`, `horizontal`, `responsive`. |
| Table | `ui/table.tsx` | `Table`, `TableHeader`, `TableRow`, `TableHead` (40 px high, 13 px), `TableCell` (56 px high). Rows highlight on hover. |
| Card | `ui/card.tsx` | The one boxed surface. `CardHeader`, `CardTitle` (18 px), `CardDescription`, `CardAction`, `CardContent`, `CardFooter`. |
| Sheet | `ui/sheet.tsx` | A panel from the right edge. `size="wide"` (75 % of the screen) for a record with a sidebar, such as a ticket. `size="half"` (50 %, at least 36 rem) for a form, a guide or a Details panel. `SheetHeader`, `SheetTitle`, `SheetDescription`. A close button sits top right. |
| Dialog | `ui/dialog.tsx` | A small question that needs one answer: the close reason on a drop to Done, issuing a ref ID, deleting a project. A form with fields goes in a sheet. |
| AlertDialog | `ui/alert-dialog.tsx` | Only through `ConfirmDialog`. |
| Alert | `ui/alert.tsx` | `default` and `destructive`. A page-level result or error, such as a key shown once. |
| Empty | `ui/empty.tsx` | Only through `TableEmpty`. |
| Switch | `ui/switch.tsx` | An on or off setting that saves at once. On is `--foreground`, never the brand color. Use it through `AlertSwitch` for alert routes. |
| Tabs | `ui/tabs.tsx` | `Tabs`, `TabsList`, `TabsTrigger`, `TabsContent`. Tabs inside one page, such as the endpoints of Custom status page. The open tab has a brand underline. The URL is not the tab: for page-level tabs use `PageTabs`. |
| Checkbox, Radio group | `ui/checkbox.tsx`, `ui/radio-group.tsx` | A checked box or radio is `--foreground`, not magenta. Option cards that need a description per choice use `ChoiceCards` or `CheckCards`. |
| Toggle group | `ui/toggle-group.tsx` | `type="single"`, `variant="outline"`. A short option set (2 to 7) that fits in a row: the chart range, a snippet language, a new ticket's column, the card or list view. |
| Select | `ui/select.tsx` | Fixed lists longer than 7, and the filter selects. |
| Others | `ui/` | `avatar`, `dropdown-menu`, `input`, `separator`, `skeleton`, `sonner`, `spinner`, `textarea`, `toggle`. Add another shadcn component when a page first needs it. |

Button rules:

- One `default` button per view. It is the primary action.
- `danger` opens a `ConfirmDialog` (revoke, delete). `destructive` is the confirm button inside it.
- `ghost` is for Cancel and quiet actions. `link` is for inline links, such as Clear filters.
- A busy submit button shows `<Spinner data-icon="inline-start" />` and is disabled.
- Toasts use `sonner` (`toast.success`, `toast.error`). The `Toaster` mounts in `main.tsx`.
- Every button shows the pointer. See 1.10.

## 2.2 Tone badges

File: `components/app/Tone.tsx`. A badge is the only place a state gets color.

| Component | Input | Output |
| --- | --- | --- |
| `StateBadge` | `kind`: `up`, `down`, `issue`, `unknown`; optional `label` | Up, Down, Issue, No data yet |
| `SeverityBadge` | `severity` | Critical (`down`), Warning (`issue`) |
| `LabelBadge` | status update label | Investigating (`down`), Identified (`issue`), Monitoring (`neutral`), Resolved (`up`) |
| `StageBadge` | `stage`, optional `ring` | Triage (`unknown`), Backlog (`neutral`), In progress (`issue`), Done (`up`) |
| `ReasonBadge` | `reason` | Fixed (`up`), Won't fix (`neutral`), Duplicate (`unknown`) |

Use `Badge variant="outline"` for a neutral fact, such as "On the status page". Use `Badge variant="neutral"` for a release's change kind (Added, Changed, Fixed, Removed) and for the Filtered badge in a page title. Use `Badge variant="up"` and `variant="unknown"` for a ref ID that is Active or Revoked.

## 2.3 Lists and tables

| Component | File | Use |
| --- | --- | --- |
| `TableCard` | `app/TableCard.tsx` | A `Card` that holds a table. Its band shows `title` (the count, such as "2 of 4 monitors") on the left and `tools` (search, filters, an action) on the right. `onClear` shows Clear filters beside the count while a search or filter narrows the list. |
| `FilterCount` | `app/TableCard.tsx` | The count with Clear filters. `TableCard` uses it. A page of cards without a table band, such as the project card view, uses it directly. |
| `PageHeader filtered` | `layout/Page.tsx` | Shows a neutral Filtered badge after the page title while a search or filter narrows the list. Resolved or All counts as filtered on the incident lists. |
| `ListFilters` | `app/ListFilters.tsx` | Search plus State and Severity selects for a `TableCard` band. A select applies at once. Props: `noun`, `filter`, `onChange`, `stateOptions`, and `selects={false}` for a list with search only. |
| `FilterMenu` | `list/FilterMenu.tsx` | One quiet filter or sort in a band: a ghost button that names the filter (Owner, Sort) and a menu of radio choices. A set filter shows its choice on the button. |
| `SortHead` | `list/SortHead.tsx` | A column heading that sorts. It is a button with an arrow for the order, and the head carries `aria-sort`. Pure rules in `lib/sort.ts`. Monitors, heartbeats, incidents and keys sort in the browser, with `?sort=&dir=`. The project list view, the cross-project Incidents and Tickets lists and the Changelog list sort on the server. |
| `LinkRow` | `app/LinkRow.tsx` | A table row that opens its page from a click anywhere on it. The first cell still holds a real link (`ROW_LINK`) for the keyboard. A click on a control inside the row keeps its own action. Cmd or Ctrl opens a new tab. |
| `useListFilter`, `keeps`, `countLabel` | `lib/useListFilter.ts`, `lib/filter.ts` | Filter state, row test and the band count text, for a list the browser filters. |
| `useServerList` | `lib/useServerList.ts` | A list the server filters and pages: search, state, severity, sort and page live in the URL and are remembered per browser. The search waits for a pause in typing. |
| `TableEmpty` | `list/TableEmpty.tsx` | The empty state, always inside the card. See 3.5. |
| `Overall` | `app/Overall.tsx` | One sentence above a list, with a tone bar on its left: "1 of 4 monitors is down." Takes a `ListSummary`. |
| `PageSearch` | `list/PageSearch.tsx` | Search beside a page title, for a page of cards rather than one table. |
| `ViewToggle` | `list/ViewToggle.tsx` | Card or list view of one page, as a two-icon `ToggleGroup`. A click on the open view keeps it. `useViewMode` remembers the choice per browser. |
| `ProjectCard`, `ProjectTable` | `app/ProjectTile.tsx` | A project as a card or a table row. The card shows the icon, name, owner, a state badge, a bar per hour of the last day with the mean uptime, four counts (Monitors, Heartbeats, Incidents, Tickets) and a foot with the description and a Status page link. Down and Issue counts take red or yellow, and open incidents take red. The whole card opens the project. Cards lift on hover. `ProjectCardSkeletons` hold the place while loading. |
| `HistoryPager` | `app/History.tsx` | "Newest" and "Older" links under a history table. |
| `Pager` | `list/Pager.tsx` | Under a list the server reads by page: "Showing 21 to 31 of 31 projects.", Previous, "Page 2 of 2", Next. Hidden when every row fits on one page. Rules in `lib/page.ts`. |
| `RefreshButton` | `app/RefreshButton.tsx` | An outline button that reads the page again now, with a spinner while it runs and no skeleton flash. Live pages (the project list, monitor and heartbeat lists and details) also read again every 30 seconds and skip it while the browser tab is hidden. |
| `Sparkline` | `app/Sparkline.tsx` | Monitors list: 24 hourly mean response times, failed hours shaded red. The line is the brand color. |
| `DueBar` | `app/DueBar.tsx` | Heartbeats list: the share of the wait before the deadline that has passed. Red when late. |
| `Skeleton` | `ui/skeleton.tsx` | The loading placeholder for a card or table. Every page load shows one. |

A table row's first cell holds the item's link and a mono second line. Secondary columns hide at small widths with `max-sm:hidden`, `max-md:hidden` or `max-lg:hidden`.

## 2.4 Detail pages

| Component | File | Use |
| --- | --- | --- |
| `StatCards` | `app/Insight.tsx` | Four numbers under the title. A 3 px bar on the left shows the tone: `up`, `down` or `plain` (`--neutral-strong`). |
| `ChartCard` | `app/Insight.tsx` | Totals, a range `ToggleGroup` (Hour, Day, Week, Month) kept in `?range=`, the SVG chart and its legend. Hover shows a guide line and a label with the bucket's time, its count and Up or Down for how long. There are no native tooltips. |
| `AvailabilityCard` | `app/Insight.tsx` | Period table with an availability bar. Bar color follows `grade()`: good `--up`, fair `--issue`, poor `--down`. |
| `Facts` | `app/Facts.tsx` | A `dl` of label and value rows. Takes `rows: [string, ReactNode][]`. |
| `Side` | `app/Side.tsx` | A titled block in a page's or a panel's right column, with a hairline under it. Used by the incident and ticket sidebars. |
| `DetailsSheet`, `DetailPart` | `app/DetailsSheet.tsx` | The Details button beside Edit on a monitor or heartbeat, and its half sheet. `DetailPart` is one titled part inside it. |
| `CodeCopy` | `app/CopyField.tsx` | A command or script to copy, under its title, on a `--paper` field. Props: `id`, `title`, `code`, `lang` (`sh`, `js` or `py`, default `sh`). `lang` colors comments, strings, keywords, flags and numbers with the `--code-*` tokens (`lib/highlight.ts`). It scrolls sideways rather than wrap. |
| `CopyButton` | `app/CopyField.tsx` | A secondary small button that copies a value and says Copied for 2 seconds. If the clipboard is blocked it selects the element named by `targetId`. `CopyField` and `CodeCopy` use it. |
| `CopyField` | `app/CopyField.tsx` | A mono value with a Copy button, for a new API key or a ping URL. Needs an `id`. |
| `ProjectBadge` | `app/ProjectBadge.tsx` | The project's icon, or its first letter on the brand color. |
| `IconPicker`, `UserPicker` | `app/` | Project icon upload and owner search in project forms and General. |

Charts are SVG. The server lays out the geometry in `checks::chart`. React draws it in `ChartSvg` (`app/Insight.tsx`). It reads the `Chart` type in `lib/types.ts`. `lib/chart.ts` (`readout`) finds the bucket under the pointer. No chart library is allowed.

## 2.5 Forms and settings

| Component | File | Use |
| --- | --- | --- |
| `SettingsPage` | `app/SettingsSection.tsx` | A settings page: `PageHeader` (`title`, `actions`), then its sections in one column. `toc` adds On this page, a sticky list of the sections at the right (from `xl`), with the section in view marked by a brand bar. |
| `SettingsSection` | `app/SettingsSection.tsx` | One section card, the same on every settings and admin page: a title and one sentence, an optional `action` at the right of the header, the fields, and a `--paper` footer with `footerNote` at the left and `footer` actions at the right. `id` is the anchor On this page links to. `onSubmit` makes it a form. `danger` marks the section that destroys data (red title and border). `flush` runs the content edge to edge, for a table or rows. |
| `SettingsSection reference` | same | A section that explains or hands out something and changes nothing, such as Custom status page. A supervisor still uses it. Without `reference`, a supervisor sees the fields disabled and no `footer` and no `action`. |
| `SeverityField` | `app/CheckForm.tsx` | Critical and Warning as two `ChoiceCards`. |
| `ChoiceField` | `app/CheckForm.tsx` | A required `Select` over a fixed list, such as a check interval in words. |
| `WhenItFails`, `PublicField` | `app/CheckForm.tsx` | The incident and alert switches of a monitor or heartbeat, and the status page checkbox. |
| `ChoiceCards` | `app/ChoiceCards.tsx` | One choice from a short set, as cards: the name, then one plain sentence. The chosen card takes a dark border and ring, not the brand color. `unavailable` shows a card dashed and disabled, with the reason in place of the sentence (Resolved on an open automatic incident). Used for Severity, a user's role and an update's label. |
| `CheckCards` | `app/ChoiceCards.tsx` | Options that turn on and off alone, as cards in a grid that wraps. A ticked card takes the same dark border. `locked` shows an option that is always on. Used for the public tabs. |
| `AlertSwitch` | `app/AlertSwitch.tsx` | An On or Off label and a `Switch` in the header of an alert section. It saves at once, shows a toast and goes back if the save fails. Off keeps the recipients. |
| `FormSheet`, `FormSheetBody` | `app/FormSheet.tsx` | The half sheet a create or edit form opens in, over its page. `FormSheetBody` takes a `title`, an optional one-line `description`, an optional `guide` button, the fields in a column that scrolls, and a footer that stays in view. Delete sits at the left of the footer. Cancel and the primary button sit at the right. The first field takes the focus. It never opens for a supervisor, even from a typed URL. |
| `GuideSheet` | `app/Guides.tsx` | A secondary button with a book icon that opens a half sheet of numbered sentences and a note. `MonitorGuide`, `HeartbeatGuide` and `IncidentGuide` use it in the page header and the form header. |
| `AlertsGuide` | `app/AlertsGuide.tsx` | A half sheet on the Alerts page: how alerts work, payload samples and delivery. |
| `ConfirmDialog` | `app/ConfirmDialog.tsx` | An `AlertDialog` for a risky action. Props: `trigger`, `title`, `description`, `action`, `onConfirm`. Cancel takes the focus. The confirm button repeats the action's label. A failed action shows a toast. |
| `NewProjectSheet` | `app/NewProjectSheet.tsx` | New project in a `FormSheet`: name, owner (super admin), homepage (required), description, icon. |
| `Writable`, `useCanEdit` | `lib/session.tsx` | A supervisor reads and changes nothing. `Writable` renders its children only for someone who may change things: wrap each create, edit and delete control in it. `useCanEdit` gives the same answer as a boolean. `FormSheet`, `ConfirmDialog` and `SettingsSection` guard this on their own. The server also refuses any write from a supervisor. |

## 2.6 Public components

| Component | File | Use |
| --- | --- | --- |
| `SystemCard` | `app/PublicStatus.tsx` | One public system as a cell of the Uptime grid: name, `StateIcon`, 90 `DayBars`, "90 days ago", the uptime, "Today", and a state line. Cells share borders in a two-column grid from 1000 px. |
| `StateIcon` | `app/PublicStatus.tsx` | A filled circle icon: check in green, cross in red, alert in yellow, dash in grey for no data yet. |
| `DayBars` | `app/PublicStatus.tsx` | One bar per day, oldest left: green, yellow for a partial day, red, or grey for no data. |
| `IncidentRow`, `IncidentList` | `app/PublicStatus.tsx` | The one public incident row: state icon, title, the newest update, its label and the time span. It opens to every update (`Updates`). It shows no author and no cause. `IncidentList` puts rows in one card with hairlines and an optional footer. Happening now, each month and the history all use it. |
| `IncidentMonthList` | `app/IncidentMonths.tsx` | A month heading and its resolved incidents, newest first. Three show. "Show all N incidents" opens the rest. A full month links to the history. A month leaves out open incidents. |
| `MonthGrid` | `app/UptimeCalendar.tsx` | One month as a Sunday-first grid of day squares with the month's uptime. |
| `WindowNav` | `app/WindowNav.tsx` | Earlier and Later arrows around a three-month label. An arrow with nowhere to go is off. |
| `ReleaseCard`, `VersionIndex`, `Neighbour` | `app/ReleaseCard.tsx` | The public Changelog: one release as a card (version, day, title, change kinds as neutral badges and a bullet list), the version list beside it with a server search (a select on a phone), and Newer and Older links. |
| `MyTickets` | `app/MyTickets.tsx` | A reporter's tickets, each with its column. It shows no description. |

## 2.7 Layout components

| Component | File | Use |
| --- | --- | --- |
| `TopBar` | `layout/TopBar.tsx` | Sticky band in the card color: the mark, the place (the open project, or a way back to the last one), the area links, the account menu. Takes `crumb` and `tabs`. |
| `PageTabs` | `layout/PageTabs.tsx` | The tab bar. The URL is the open tab. The open tab has a brand underline. A count shows beside a tab when above zero. `alarm` makes the count red, for something broken. A count without `alarm` is neutral. `center` puts the tabs in the middle, as on the public pages. `end` holds a link that leaves the area. It scrolls sideways on a phone. |
| `SideNav`, `SidebarFrame` | `layout/SideNav.tsx` | Left navigation for Settings and Admin. The open item is a raised card with a 2 px brand bar. A scrolling row on a phone. |
| `PageFrame`, `Crumbs`, `PageHeader` | `layout/Page.tsx` | Content column, breadcrumbs, and the title row (`lead`, `title`, `filtered`, `meta`, `actions`). |
| `ThemeProvider`, `ThemeToggle` | `layout/` | Theme state and the toggle button. |

## 2.8 Tickets

| Component | File | Use |
| --- | --- | --- |
| `TicketBoard` | `app/TicketBoard.tsx` | One column per stage, in a row that scrolls sideways. A column has a fixed height and scrolls inside. Its header shows a `StageRing`, the name and a count pill in the column's color. `notes` puts one line of help under a column name. `footer` renders the foot of a column that holds more than it shows (`ShowMore`). With `onMove`, a card drags to another column. The card itself follows the pointer and the target column shows a dashed drop slot. The drop names the whole column. Used by the team board and the public Issues tab (no drag). |
| `TicketCard` | `app/TicketBoard.tsx` | The number, an `aside` such as the age, the title and a `meta` line. |
| `StageRing` | `app/TicketBoard.tsx` | A ring in front of a column name: dashed grey for Triage, empty `--neutral-strong` for Backlog, half `--issue` for In progress, full `--up` for Done. |
| `ShowMore` | `app/TicketBoard.tsx` | A ghost button at the foot of a column that loads the next page of that column. Done also links to `/tickets/done`, a paged table. |
| `TicketPanel` | `app/TicketPanel.tsx` | A ticket over the board, in a wide `Sheet` at `tickets/:number`. The header has the title and number, the `StageBadge` and `ReasonBadge`, who filed it and when, the ref ID line (`rf_…` plus its last 4, Active or Revoked, Revoke) and the public title in a raised box with a pencil to edit in place. The body has the report as a comment card, then a sidebar with the Column picker, the public preview and the dates. |
| `ColumnPicker`, `PublicPreview`, `PublicTitle` | `app/TicketSide.tsx` | The panel's sidebar parts. Column is a `Select` that saves at once. Done adds a required Close reason `Select`. |
| `NewTicketSheet` | `app/NewTicketSheet.tsx` | The team files a ticket: title, description and a column (not Done). No reporter, no ref ID. |
| `ConnectApp` | `app/ConnectApp.tsx` | A secondary button that opens a half sheet of numbered steps with `curl` calls written for this project: make a key, issue a ref ID, file a ticket, list a reporter's tickets, link the hosted form, hear about new tickets. |
| `IssueRefId` | `app/IssueRefId.tsx` | A dialog that issues a ref ID by hand. |

## 2.9 Illustrations

Files: `components/brand/Mascot.tsx`, `components/brand/Playground.tsx`, `components/app/footers/`. Rules for Moni are in 1.11.

| Component | Use |
| --- | --- |
| `Mascot` | Moni, drawn at (x, y) inside a scene's SVG. Props: `scale`, `happy`, `flip`, `children`. `ARM` is the class for an arm. |
| `PrimitiveFooter` | The footer for a primitive's pages. `kind` is `monitors`, `heartbeats`, `incidents`, `tickets` or `changelog`. `footerFor` in `lib/primitiveFooter.ts` picks it from the path. |
| `Frame` | The footer shell: a full-width band, 200 px high (150 px on a phone), with a 560 by 200 scene in the middle and a textured edge on each side. |
| `Window`, `Flow`, `TextLine` | Scene parts. `Window` is a card with a title bar. `Flow` is a dashed line with an arrowhead that marches toward its end: brand for what Monita sends, green (`answer`) for a reply. `TextLine` is a grey bar that stands for text. |
| `Playground` | The sign-in scene: Monis at play along the bottom edge. |
| `PeekingMoni` | Moni looking over the top edge of the sign-in card. |
