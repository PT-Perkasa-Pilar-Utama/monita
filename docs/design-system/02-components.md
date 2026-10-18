# 2. Components

Order of choice: a shadcn component in `web/src/components/ui/`, then a Monita component below, then new markup. A new Monita component goes in `components/app/` and gets a row here.

## 2.1 shadcn components, as Monita restyles them

| Component | File | Variants and rules |
| --- | --- | --- |
| Button | `ui/button.tsx` | `variant`: `default` (brand magenta), `secondary`, `outline`, `ghost`, `destructive`, `danger`, `link`. `size`: `default` (36 px), `sm` (32 px), `lg` (44 px), `icon`, `icon-sm`. Pill shape. |
| Badge | `ui/badge.tsx` | `variant`: `up`, `down`, `issue`, `unknown`, `neutral`, `destructive`, `outline`, `secondary`, `default`. A tone pill: tinted fill, colored word, no dot. |
| Label | `ui/label.tsx` | `required` adds a red asterisk. The control also gets `aria-required`. |
| Field | `ui/field.tsx` | `FieldGroup`, `Field`, `FieldLabel`, `FieldError`, `FieldSet`, `FieldLegend`. `Field orientation`: `vertical`, `horizontal`, `responsive`. |
| Table | `ui/table.tsx` | `Table`, `TableHeader`, `TableRow`, `TableHead` (40 px high, 13 px), `TableCell` (56 px high). Rows highlight on hover. |
| Card | `ui/card.tsx` | The one boxed surface. `CardHeader`, `CardTitle`, `CardAction`, `CardContent`, `CardFooter`. |
| Dialog | `ui/dialog.tsx` | Quick creates and edits: New project, Invite tech lead, New API key, Post update. |
| AlertDialog | `ui/alert-dialog.tsx` | Only through `ConfirmDialog`. |
| Alert | `ui/alert.tsx` | `default` and `destructive`. A page-level result or error, such as a key shown once. |
| Empty | `ui/empty.tsx` | Only through `TableEmpty`. |
| Toggle group | `ui/toggle-group.tsx` | `type="single"`, `variant="outline"`. A short option set (2 to 7) in a form. |
| Radio group | `ui/radio-group.tsx` | Option cards that need a description per choice, such as `SeverityField`. |
| Select | `ui/select.tsx` | Fixed lists longer than 7, and the filter selects. |
| Others | `ui/` | `avatar`, `checkbox`, `dropdown-menu`, `input`, `separator`, `skeleton`, `sonner`, `spinner`, `textarea`, `toggle`. Add another shadcn component when a page first needs it. |

Button rules:

- One `default` button per view. It is the primary action.
- `danger` opens a `ConfirmDialog` (revoke, delete). `destructive` is the confirm button inside it.
- `ghost` is for Cancel and quiet actions. `link` is for inline links.
- A busy submit button shows `<Spinner data-icon="inline-start" />` and is disabled.
- Toasts use `sonner` (`toast.success`, `toast.error`). The `Toaster` mounts in `main.tsx`.

## 2.2 Tone badges

File: `components/app/Tone.tsx`. A badge is the only place a state gets color.

| Component | Input | Output |
| --- | --- | --- |
| `StateBadge` | `kind`: `up`, `down`, `issue`, `unknown`; optional `label` | Up, Down, Issue, Waiting |
| `SeverityBadge` | `severity` | Critical (`down`), Warning (`issue`) |
| `LabelBadge` | status update label | Investigating (`down`), Identified (`issue`), Monitoring (`neutral`), Resolved (`up`) |

Use `Badge variant="outline"` for a neutral fact, such as "On the status page". Use `Badge variant="neutral"` for a finished ticket's close reason.

## 2.3 Lists and tables

| Component | File | Use |
| --- | --- | --- |
| `TableCard` | `app/TableCard.tsx` | A `Card` that holds a table. Its band shows `title` (the count, such as "2 of 4 monitors") on the left and `tools` (search, filters, an action) on the right. |
| `ListFilters` | `app/ListFilters.tsx` | Search plus State and Severity selects for a `TableCard` band. A select applies at once. Props: `noun`, `filter`, `onChange`, `stateOptions`. |
| `useListFilter`, `keeps`, `countLabel` | `lib/useListFilter.ts`, `lib/filter.ts` | Filter state, row test and the band count text. |
| `TableEmpty` | `list/TableEmpty.tsx` | The empty state, always inside the card. See 3.5. |
| `Overall` | `app/Overall.tsx` | One sentence above a list, with a tone bar on its left: "1 of 4 monitors is down." Takes a `ListSummary`. |
| `PageSearch` | `list/PageSearch.tsx` | Search beside a page title, for a page of cards rather than one table. |
| `HistoryPager` | `app/History.tsx` | "Newest" and "Older" links under a history table. |
| `Pager` | `list/Pager.tsx` | Under a list the server reads by page: "Showing 21 to 31 of 31 projects.", Previous, "Page 2 of 2", Next. Hidden when every row fits on one page. Rules in `lib/page.ts`. |
| `Sparkline` | `app/Sparkline.tsx` | Monitors list: 24 hourly mean response times, failed hours shaded red. The line is the brand color. |
| `DueBar` | `app/DueBar.tsx` | Heartbeats list: the share of the wait before the deadline that has passed. Red when late. |
| `Skeleton` | `ui/skeleton.tsx` | The loading placeholder for a card or table. |

A table row's first cell holds the item's link (`font-medium hover:underline`) and a mono second line. Secondary columns hide at small widths with `max-sm:hidden`, `max-md:hidden` or `max-lg:hidden`.

## 2.4 Detail pages

| Component | File | Use |
| --- | --- | --- |
| `StatCards` | `app/Insight.tsx` | Four numbers under the title. A 3 px bar on the left shows the tone: `up`, `down` or `plain` (`--neutral-strong`). |
| `ChartCard` | `app/Insight.tsx` | Totals, a range `ToggleGroup` (Hour, Day, Week, Month) kept in `?range=`, the SVG chart and its legend. |
| `AvailabilityCard` | `app/Insight.tsx` | Period table with an availability bar. Bar color follows `grade()`: good `--up`, fair `--issue`, poor `--down`. |
| `Facts` | `app/Facts.tsx` | A `dl` of label and value rows. Takes `rows: [string, ReactNode][]`. |
| `CopyField` | `app/CopyField.tsx` | A mono value with a Copy button, for a new API key or a ping URL. Needs an `id`. |
| `ProjectBadge` | `app/ProjectBadge.tsx` | The project's icon, or its first letter on the brand color. |
| `IconPicker`, `UserPicker` | `app/` | Project icon upload and owner search in project dialogs and General. |

Charts are SVG. The server lays out the geometry in `checks::chart`. React draws it in `ChartSvg` (`app/Insight.tsx`). It reads the `Chart` type in `lib/types.ts`. No chart library is allowed.

## 2.5 Forms

| Component | File | Use |
| --- | --- | --- |
| `SeverityField` | `app/CheckForm.tsx` | Critical and Warning as two radio cards. The checked card takes a brand tint. |
| `ChoiceField` | `app/CheckForm.tsx` | A required `Select` over a fixed list of seconds, such as a check interval. |
| `FormAside` | `app/CheckForm.tsx` | The card beside a create or edit form. It lists, in numbered sentences, what the thing will do. |
| `ConfirmDialog` | `app/ConfirmDialog.tsx` | An `AlertDialog` for a risky action. Props: `trigger`, `title`, `description`, `action`, `onConfirm`. Cancel takes the focus. The confirm button repeats the action's label. A failed action shows a toast. |
| `NewProjectDialog` | `app/NewProjectDialog.tsx` | New project: name, owner (super admin), description, icon. |

## 2.6 Public components

| Component | File | Use |
| --- | --- | --- |
| `SystemRow` | `app/PublicStatus.tsx` | One public system: state badge, name, 90-day uptime and one bar per day. |
| `IncidentCard` | `app/PublicStatus.tsx` | A public incident as `details`. It is open while the incident is open. It shows no author and no cause. |
| `TicketBoard`, `TicketCard` | `app/TicketBoard.tsx` | Columns of ticket cards, one per stage. Used by the team board and the public board. |

Stage rings in `TicketBoard`: Triage is dashed, Backlog is empty, In progress is half `--issue`, Done is `--neutral-strong`.

## 2.7 Layout components

| Component | File | Use |
| --- | --- | --- |
| `TopBar` | `layout/TopBar.tsx` | Sticky band: mark, optional project crumb, Projects, Admin menu (super admin), Docs, account menu. Takes `crumb` and `tabs`. |
| `PageTabs` | `layout/PageTabs.tsx` | The tab bar. The URL is the open tab. The open tab has a brand underline. A count shows when above zero. It scrolls sideways on a phone. |
| `SideNav`, `SidebarFrame` | `layout/SideNav.tsx` | Left navigation for Settings and Admin. The open item is a raised card with a 2 px brand bar. A scrolling row on a phone. |
| `PageFrame`, `Crumbs`, `PageHeader` | `layout/Page.tsx` | Content column, breadcrumbs, and the title row (`lead`, `title`, `meta`, `actions`). |
| `ThemeProvider`, `ThemeToggle` | `layout/` | Theme state and the toggle button. |
