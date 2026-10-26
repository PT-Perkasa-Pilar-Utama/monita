# 1. Foundations

All tokens live in `web/src/index.css`. Light values sit in `:root`. Dark values sit in `.dark`. Tailwind reads them through `@theme inline`, so `bg-primary`, `text-down` and `fill-up` work. A component never holds a hex value.

## 1.1 Themes

| Theme | Class on `<html>` | When |
| --- | --- | --- |
| Light | `light` | Saved choice, or OS light |
| Dark | `dark` | Saved choice, or OS dark |

- `static/theme.js` adds the class before the first paint. It reads `localStorage` key `monita-theme`, then `prefers-color-scheme`.
- `ThemeProvider` (`components/layout/ThemeProvider.tsx`) keeps the class in sync. No saved choice means the OS setting.
- The app has a Toggle theme item in the account menu. Public and sign-in pages have `ThemeToggle`.
- Both themes keep body and muted text readable. Muted text never fades into the background.

## 1.2 Color tokens

| Token | Use |
| --- | --- |
| `--background` | Page, top band |
| `--foreground` | Titles and body text |
| `--card`, `--card-foreground` | Cards, inputs, dialogs |
| `--popover` | Menus and popovers |
| `--paper` | Table bands, aside cards, board columns, code fields |
| `--secondary` | Secondary buttons |
| `--muted`, `--muted-foreground` | Tracks, hover rows, captions, table heads, labels |
| `--accent` | A neutral hover fill. It is not the brand color. |
| `--border` | Every hairline |
| `--input` | Input borders and the chart axis |
| `--ring` | Focus ring (brand magenta at 45 %) |
| `--primary`, `--primary-foreground` | The brand: magenta `#c4258f` light, `#f25cbf` dark. Text on it is white in both themes. |
| `--primary-raised`, `--destructive-raised` | Dark mode only. A solid brand or danger fill with text on it (primary and destructive buttons, the default badge, a project's letter badge) uses this darker gradient, because white text on `#f25cbf` has a contrast of only 2.8:1. The middle of each gradient gives white text at least 4.5:1. |
| `--link` | Link text: `#a61f79` light, `#f25cbf` dark |
| `--destructive` | Destructive button, required asterisk, field errors |
| `--code-comment`, `--code-string`, `--code-keyword`, `--code-flag` | Syntax colors in `CodeCopy` snippets (GitHub light and dark). A number uses `--code-flag`. They are not states: never reuse `--up`, `--down` or `--issue` for code. |
| `--mascot-body`, `--mascot-line`, `--mascot-ink`, `--mascot-shine`, `--mascot-cheek` | Moni, the mascot. Set once in `:root`, not under `.dark`, so Moni looks the same in both themes. See 1.11. |

Checked checkboxes and radios, the `Switch` when on, and a chosen option card use `--foreground`, not the brand color. A choice is not the brand.
The magenta accent means the brand only: the mark, the primary button, focus, links and the chart line. It never marks a state.

## 1.3 State colors

State colors carry data. They are the only colors outside the neutrals and the brand.

| Token | Dark | Means |
| --- | --- | --- |
| `--up` | `#4cb782` | Monitor up, heartbeat on time, Resolved, Ping received |
| `--down` | `#eb5757` | Down, missed, Critical, Investigating, Failed |
| `--issue` | `#f2c94c` | Open manual incident, Warning, Identified, In progress |
| `--unknown` | `#8a8f98` | No data yet, Triage, Revoked, Duplicate |
| `--neutral-strong` | `#a9aeb6` | Backlog, the Monitoring label, Won't fix, and the `plain` stat bar |
| `--series-1` to `--series-5` | magenta, blue, violet, teal, brown | The Overview chart's five response-time lines. Never red, green or yellow, which mark a state. |

Ticket stages and close reasons are state pills (`StageBadge`, `ReasonBadge`, `components/app/Tone.tsx`):

| Stage | Pill | Close reason | Pill |
| --- | --- | --- | --- |
| Triage | `unknown` (grey) | Fixed | `up` (green) |
| Backlog | `neutral` | Won't fix | `neutral` |
| In progress | `issue` (yellow) | Duplicate | `unknown` (grey) |
| Done | `up` (green) | | |

A state pill is a tinted fill and a colored word, with no dot. The ring in front of a column name (`StageRing`) follows the same colors.

Light values are darker so text keeps its contrast: `--up` `#23784e`, `--issue` `#8a6300`.

## 1.4 Type

| Role | Font | Size and weight |
| --- | --- | --- |
| Body | Inter | 14 px, line height 1.5 |
| Page title (`PageHeader`) | Inter | 24 px, 32 px from `sm`, bold, tight tracking |
| Card title (`CardTitle`) | Inter | 18 px, semibold, tight tracking. The same on every card, settings section and sheet card. |
| Sheet title | Inter | 20 px, semibold. A ticket panel title is 24 px. |
| Section heading (About, Systems, Past incidents) | Inter | 18 px, semibold |
| Sidebar block title (`Side`) | Inter | 13 px, semibold, `--muted-foreground` |
| Status headline (public) | Inter | 20 px, semibold |
| Table head | Inter | 13 px, medium, `--muted-foreground` |
| Table band title | Inter | 13 px, semibold |
| Caption | Inter | 12 px, `--muted-foreground` |
| URLs, tokens, codes, times, counts | Geist Mono (`font-mono`) | 12 to 13 px |

- Fonts load from `static/fonts/` through `@font-face` in `index.css`.
- Numbers in tables use `tabular-nums`.
- No heading is uppercase. No label sits above a title as an eyebrow.

## 1.5 Space and layout

- Tailwind's 4 px scale. Use `gap` on flex and grid parents.
- `PageFrame` pads the page at 16, 32 and 48 px (`px-4`, `sm:px-8`, `lg:px-12`) and caps the column at `max-w-app` (1440 px).
- `--bar-h` is the height of the sticky top band: 57 px, or 101 px when the header has tabs. Sticky side panels use `top-[calc(var(--bar-h)+24px)]`.
- A page with a side column splits at `lg` or `xl`: a flexible column and a fixed one, 18 rem for About and the incident sidebar, 13 rem for On this page, 17 rem for the ticket panel sidebar.
- A side `Sheet` has two widths. `wide` is 75 % of the screen, for a record with a sidebar such as a ticket. `half` is 50 % with a 36 rem minimum, for a form, a guide or a Details panel. Both are full width on a phone.

## 1.6 Shape

| Radius | Used on |
| --- | --- |
| Full | Count pills, tab counts, `Switch`, avatar, radio |
| `rounded-xl` (12 px) | Cards, dialogs, board columns, status headline |
| `rounded-lg` (8 px) | Buttons, inputs, selects, toggles, menu rows in the nav, ticket cards, copy field, option cards |
| `rounded-md` (6 px) | Badges, project badge |

A button has the same 8 px corner as an input, so a button beside a field lines up. Buttons are not pills.

## 1.7 Elevation

Depth is one surface step plus a hairline. Cards have no shadow. The one exception is the project card in the card view: on hover it lifts 2 px and takes `shadow-md` (no lift under reduced motion). The top band is the card color with `shadow-xs`. Floating things (dialogs, menus, popovers) take their shadow from the shadcn component. A raised nav item uses `shadow-xs` and `ring-1 ring-border`. The top band is opaque. No glow, no gradient except the chart fill.

## 1.8 Motion

- Buttons, tabs and nav links fade color in 150 ms (`transition-colors duration-150`).
- Dialogs, menus and sheets use the shadcn enter and exit animation from `tw-animate-css`. A sheet slides in from the right in 200 ms.
- Nothing in the product UI moves on scroll or pulses. The only looping motion is in illustrations: Moni, the footer scenes and the sign-in playground (1.11).
- `prefers-reduced-motion: reduce` cuts every animation and transition to 0.01 ms and runs each animation once (`animation-iteration-count: 1`), so every loop in an illustration stops (`web/src/index.css`).

## 1.9 Icons

- Phosphor, through the barrel `web/src/components/icons/index.ts`. Never import `@phosphor-icons/react` or another icon pack anywhere else.
- The barrel re-exports icons under the names code already uses, for example `Activity` for Phosphor `Pulse`. To swap the pack, change the barrel only.
- An icon in a button carries `data-icon="inline-start"` or `data-icon="inline-end"`. The button sizes it and spaces it.
- An icon-only button has an `aria-label`.
- The brand mark is `components/brand/Mark.tsx`. It takes its color from `currentColor`.

## 1.10 Pointer

Anything a click acts on shows the pointer. One base rule in `index.css` sets `cursor: pointer` on `button:not(:disabled)`, `[role="button"]:not([aria-disabled="true"])`, `label[for]`, `summary` and `select`. Tailwind 4 leaves buttons on the default arrow, so a component never adds `cursor-pointer` to a button. A clickable table row (`LinkRow`) and an option card set it themselves. A disabled control keeps the arrow.

## 1.11 Mascot

Moni is the mascot (`components/brand/Mascot.tsx`). It is decoration. It never carries data and never takes input.

| Rule | Detail |
| --- | --- |
| Built from the logo | A ring is the round body, the pulse line is the mouth, the antenna listens. Feet are two ovals. |
| Size | Drawn around (0, 0) with a body radius of 46, scaled by the scene. |
| Colors | Only the `--mascot-*` tokens: body, line, ink, shine, cheek. They hold one value in both themes, so Moni never changes face in dark mode. |
| Face | Two ink eyes with a white shine and pink cheeks. `happy` swaps the eyes for smiling arcs. Moni faces left. `flip` faces right. |
| Arms | The scene draws them: a 4 px round stroke in `--mascot-line` (`ARM`). |
| Color meaning | The brand color is Moni and the flow lines Monita sends. Green marks a success: a 200 OK, a passing check, Resolved, an answer. Grey marks waiting. Illustrations never use red or yellow. |
| Things around Moni | Neutral tokens only (`card`, `input`, `muted`, `muted-foreground`). No hex value. |
| Motion | The antenna ball beats, the eyes blink, dashed flow lines march. Each loop is an `animate-[monita-*]` keyframe in `index.css`. Reduced motion stills all of them. |
| Access | Every illustration is `aria-hidden`. It holds no text a reader needs. |

Where Moni appears: the footer under each primitive's pages, and the sign-in playground. See 3.8.
