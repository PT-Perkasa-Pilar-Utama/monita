# 1. Foundations

All tokens live in `web/src/index.css`. Light values sit in `:root`. Dark values sit in `.dark`. Tailwind reads them through `@theme inline`, so `bg-primary`, `text-down` and `fill-up` work. A component never holds a hex value.

## 1.1 Themes

| Theme | Class on `<html>` | When |
| --- | --- | --- |
| Light | `light` | Saved choice, or OS light |
| Dark | `dark` | Saved choice, or OS dark |

- `static/theme.js` adds the class before the first paint. It reads `localStorage` key `monita-theme`, then `prefers-color-scheme`.
- `ThemeProvider` (`components/layout/ThemeProvider.tsx`) keeps the class in sync. No saved choice means the OS setting.
- The app has a theme switch in the account menu. Public and sign-in pages have `ThemeToggle`.
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
| `--primary`, `--primary-foreground` | The brand: magenta `#c4258f` light, `#f25cbf` dark |
| `--link` | Link text: `#a61f79` light, `#f25cbf` dark |
| `--destructive` | Destructive button, required asterisk, field errors |

The magenta accent means the brand only: the mark, the primary button, focus, links and the chart line. It never marks a state.

## 1.3 State colors

State colors carry data. They are the only colors outside the neutrals and the brand.

| Token | Dark | Means |
| --- | --- | --- |
| `--up` | `#4cb782` | Monitor up, heartbeat on time, Resolved, Ping received |
| `--down` | `#eb5757` | Down, missed, Critical, Investigating, Failed |
| `--issue` | `#f2c94c` | Open manual incident, Warning, Identified, In progress |
| `--unknown` | `#8a8f98` | No data yet, Waiting, Revoked |
| `--neutral-strong` | `#a9aeb6` | A finished stage (Done) and the Monitoring label |

Light values are darker so text keeps its contrast: `--up` `#23784e`, `--issue` `#8a6300`.

## 1.4 Type

| Role | Font | Size and weight |
| --- | --- | --- |
| Body | Inter | 14 px, line height 1.5 |
| Page title (`PageHeader`) | Inter | 24 px, 32 px from `sm`, bold, tight tracking |
| Card title | Inter | 15 px, semibold |
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
- Wide pages split at `xl` into `minmax(0,2fr)` and `minmax(280px,1fr)`.

## 1.6 Shape

| Radius | Used on |
| --- | --- |
| Full | Buttons, count pills, tab count |
| `rounded-xl` (12 px) | Cards, dialogs, board columns, status headline |
| `rounded-lg` (8 px) | Inputs, selects, menu rows in the nav, ticket cards, copy field |
| `rounded-md` (6 px) | Badges, project badge |

## 1.7 Elevation

Depth is one surface step plus a hairline. Cards have no shadow. Floating things (dialogs, menus, popovers) take their shadow from the shadcn component. A raised nav item uses `shadow-xs` and `ring-1 ring-border`. The top band is opaque. No glow, no gradient except the chart fill.

## 1.8 Motion

- Buttons, tabs and nav links fade color in 150 ms (`transition-colors duration-150`).
- Dialogs and menus use the shadcn enter and exit animation from `tw-animate-css`.
- Nothing moves on scroll. No pulsing dots.
- `prefers-reduced-motion: reduce` cuts every animation and transition to nothing (`web/src/index.css`).

## 1.9 Icons

- Phosphor, through the barrel `web/src/components/icons/index.ts`. Never import `@phosphor-icons/react` or another icon pack anywhere else.
- The barrel re-exports icons under the names code already uses, for example `Activity` for Phosphor `Pulse`. To swap the pack, change the barrel only.
- An icon in a button carries `data-icon="inline-start"` or `data-icon="inline-end"`. The button sizes it and spaces it.
- An icon-only button has an `aria-label`.
- The brand mark is `components/brand/Mark.tsx`. It takes its color from `currentColor`.
