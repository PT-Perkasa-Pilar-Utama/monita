# 3. Patterns

## 3.1 Shells

| Shell | File | Structure |
| --- | --- | --- |
| Project | `layouts/ProjectLayout.tsx` | `TopBar` (project crumb, `PageTabs`) then `PageFrame` then the page |
| App | `layouts/AppLayout.tsx` | `TopBar` then `PageFrame`. Projects and Admin pages. |
| Admin | `layouts/AdminLayout.tsx` | `SidebarFrame` with `SideNav` (Users, System). A non-admin gets the not-found page. |
| Settings | `layouts/SettingsLayout.tsx` | `SidebarFrame` with `SideNav` (General, Alerts, API keys, Allowlist) |
| Public | `layouts/PublicLayout.tsx` | Sticky header (project name, `ThemeToggle`, `PageTabs`), `PageFrame`, footer |
| Bare | `layouts/BareLayout.tsx` | Sign-in and invite: the brand, one card (max 420 px), a footer, `ThemeToggle` |

Under 768 px (`md`) a side navigation becomes one scrolling row. The tab bar scrolls sideways. Area links in the top bar show icons only.

## 3.2 Navigation

| Place | Items |
| --- | --- |
| Top bar | Mark and project crumb, Projects, Admin (super admin: Users, System), Docs, account menu |
| Account menu | Who is signed in, All projects, theme switch, Sign out |
| Project tabs | Overview, Monitors, Heartbeats, Incidents, Tickets, Settings. Status page sits at the right end and leaves the app for `/status/{slug}`. |
| Settings side nav | General, Alerts, API keys, Allowlist |
| Public tabs | Status (also History), Known issues, Report a bug, My tickets |

Only Settings and Admin have a side navigation. Every list stays full width. Counts show beside Monitors, Heartbeats, Incidents and Tickets when above zero.

## 3.3 Page recipes

All pages open with `PageHeader`. A detail or form page puts `Crumbs` above it.

| Page | Recipe |
| --- | --- |
| Projects | `PageHeader` (`PageSearch`, New project) then a grid of project cards. New project opens `NewProjectDialog`. |
| Overview | `PageHeader` (`ProjectBadge` lead, New monitor, Report incident) then two columns: area tiles and an Open incidents `TableCard`, beside a sticky About `aside`. |
| List (monitors, heartbeats, incidents) | `PageHeader` (title, primary action) then `Overall`, then `TableCard` (count and `ListFilters`) holding a `Table`. Monitor rows carry a `Sparkline`. Heartbeat rows carry a `DueBar`. |
| Detail (monitor, heartbeat) | `Crumbs`, `PageHeader` (title, `meta` badges, Edit), `StatCards`, `ChartCard`, then `AvailabilityCard` beside a Settings card of `Facts`, then a checks `TableCard` with `HistoryPager`. |
| Detail (incident) | `Crumbs`, `PageHeader` (Post update dialog), a Facts card at left and a Timeline `TableCard` at right. |
| Form with aside | `Crumbs`, `PageHeader`, then a `Card` form and `FormAside` in a grid. Footer: Delete (edit only) at the left; Cancel and Save at the right. |
| Settings page | `SidebarFrame`, `PageHeader`, then one or more `Card`s (General, Alerts, Allowlist) or one `TableCard` (API keys, with New API key as a `Dialog`). |
| Admin Users | `PageHeader` (Invite tech lead) then a `TableCard`. Invite is a `Dialog`. Disable goes through `ConfirmDialog`. |
| System | `PageHeader` then cards: email, server, limits (one `Field` per limit), fixed caps. |
| Ticket board | `PageHeader` (Public board) then `TicketBoard` with Triage, Backlog, In progress and Done. |
| Ticket detail | `Crumbs`, `PageHeader`, then stage and close reason in `ToggleGroup`s. |
| Public status | `PageHeader`, a headline sentence with a tone border, then a grid: `SystemRow`s at left; Active incidents and Past 7 days (`IncidentCard`) at right. History shows `IncidentCard`s under month headings. |
| Public known issues | `PageHeader`, then `TicketBoard` with Backlog, In progress and Done. A card shows title and number only. |
| Report a bug | `Card` form with a required ref ID. |
| Sign-in, invite | `BareLayout` and one `Card`. |

## 3.4 Forms

- Build with `FieldGroup`, `Field`, `FieldLabel`, `FieldError`. Wrap a related set in `FieldSet` and `FieldLegend`.
- Put `data-invalid` on `Field` and `aria-invalid` on the control when a field has an error.
- A required field uses `<FieldLabel required>` and `aria-required` on the control. There are no "(optional)" labels. An unmarked field is optional.
- A needed hint goes in the label text. A field has no subtitle and no `FieldDescription`.
- A refused form returns 422 with `error.fields`. The page reads `ApiError.fields[name]` (`lib/api.ts`) and shows it in `FieldError` under that field. A message with no field shows in an `Alert` or a toast.
- An option set of 2 to 7 uses `ToggleGroup` or `RadioGroup` cards. A longer set uses `Select`.
- A form that can save keeps one primary `Button` in the footer. A busy button is disabled and shows a `Spinner`.
- Forms use `noValidate`. The server validates.

## 3.5 States

| Case | Component | Text |
| --- | --- | --- |
| Loading | `Skeleton` | None |
| Nothing yet | `TableEmpty` with `title` and at most one sentence in `description` | "Add your first monitor." plus one sentence on what a monitor does |
| A filter hides every row | `TableEmpty filtered noun="monitors" onClear` | "No monitors match these filters." and a Clear filters button |
| Risky action | `ConfirmDialog` | Title asks the question. Description says what is lost. The button repeats the action. |
| Result of an action | `toast.success` or `toast.error` | One sentence |
| A secret shown once | `Alert` with `CopyField` | "Key created. Copy it now: Monita shows it only once." |

`TableEmpty` sits inside the list's `TableCard` or `Card`. It replaces the table.

## 3.6 Copy rules

- Write whole sentences. Do not stack fragments such as "No X, no Y" or "One X, one Y".
- State what is true and literal. Use no metaphors.
- Say "tech lead", not "TL". Say "ticket board", not "kanban".
- Name things by what a tech lead knows: "Ping URL", not "heartbeat token endpoint".
- A button says what happens: "Save heartbeat", "Revoke key", "Post update".
- An error says what to do: "Enter a name, for example nightly-settlement."
- Use no eyebrow label, no uppercase label and no dot separator between facts. Separate facts with spacing or a comma.
- Times in lists are relative ("6m ago"). Times on detail pages are absolute UTC.
- Durations: `6m 18s`, `2h 4m`, `3d 2h`. Latency: `182 ms`. Uptime: two decimals on the status page (`99.91%`).

## 3.7 Do and don't

| Do | Don't |
| --- | --- |
| Put state color in a `Badge` tone pill | Put a decorative dot or a dot before a label |
| Put a list's count in its `TableCard` band | Put an uppercase label above a title |
| Use the brand color for the mark, the primary button, focus, links and the chart line | Use the brand color to mark a state |
| Use `--neutral-strong` for a finished stage | Use green for a finished stage |
| Import icons from `components/icons` | Import an icon pack directly |
| Use tokens (`bg-primary`, `text-down`) | Write a hex value in a component |
| Use one `default` button per view | Put two brand buttons side by side |
| Separate groups with hairlines | Add a shadow to a flat card |
| Draw charts as SVG from `checks::chart` | Add a chart library |
| Keep muted text readable in both themes | Use grey text that fades into the background |
