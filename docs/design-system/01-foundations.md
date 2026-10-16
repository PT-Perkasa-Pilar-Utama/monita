# 1. Foundations

All values live as tokens on `:root` in `static/app.css`. No color literal appears outside `:root`. Tints are `color-mix()` from tokens.

## 1.1 Schemes

| Scheme | When | Base values |
| --- | --- | --- |
| Dark (default) | OS dark, or no preference | bg `#08090a`, bg-2 `#0f1011`, fg `#f7f8f8`, fg-muted `#8a8f98` |
| Daylight | OS light, or `data-theme="light"` | bg `#f7f8f8`, bg-2 `#ffffff`, fg `#16171a`, fg-muted `#676b73` |

Only the base tokens change per scheme. Every derived token follows by itself.

## 1.2 Color tokens

| Token | Use |
| --- | --- |
| `--bg` | Page, sidebar |
| `--bg-2` | Panels, cards, inputs, dialogs |
| `--bg-3` | Secondary buttons, code fields |
| `--fg` | Titles, active nav, values |
| `--fg-2` | Body text |
| `--fg-muted` | Nav links, labels in key-value lists, help |
| `--fg-3` | Captions, IDs, timestamps, table headers |
| `--border` | Every hairline (8 % fg; 10 % in Daylight) |
| `--border-input` | Input borders (12 % fg) |
| `--rule` | Opaque section dividers |
| `--f2`, `--f4`, `--f6` | Row hover, nav hover, active nav |
| `--accent` | Indigo `#5e6ad2`: focus ring, accent button, Monitoring label, Done stage |

## 1.3 Status colors

Colour is data. These are the only colors outside the neutrals and the accent.

| Token | Value (dark) | Means |
| --- | --- | --- |
| `--up` | `#4cb782` green | Monitor up, heartbeat on time, Resolved |
| `--down` | `#eb5757` red | Down, missed, Critical, Investigating |
| `--issue` | `#f2c94c` yellow | Open manual incident, Warning, Identified, In progress |
| `--unknown` | `--fg-3` | No data yet |

Rule: color sits in the icon or the dot. The word next to it stays neutral, except a down state, whose word turns red so a scan finds it.

## 1.4 Type

| Role | Font | Size / weight |
| --- | --- | --- |
| UI (admin) | Inter | 13 px / 400, labels 510, titles 590 |
| Page title | Inter | 24 px / 510, tracking -0.022em |
| Public body | Inter | 15 px / 400, line height 1.6 |
| Public title | Inter | 32 px / 510 |
| Caption, ID, time, table head | Geist Mono | 11 px / 400, uppercase |
| URLs, tokens, codes | Geist Mono | 12 px |

Scale: 11, 12, 13, 15, 20, 24, 32. Never above weight 600. Never uppercase a heading.

## 1.5 Space

Base unit 4 px. Tokens `--s1` to `--s8`: 4, 8, 12, 16, 24, 32, 48, 72. Use `gap` on flex and grid parents, not margins on children.

## 1.6 Shape

| Radius | Token | Used on |
| --- | --- | --- |
| 9999 px | `--r-pill` | Buttons, chips, tabs |
| 12 px | `--r-lg` | Panels, dialogs, status headline |
| 9 px | literal in `.comment` | Comment cards |
| 8 px | `--r` | Inputs, cards, code fields |
| 6 px | `--r-row` | Sidebar rows |
| 4 px | `--r-sm` | Inline code, project badge |

## 1.7 Elevation

Depth is one surface step plus a hairline. Only floating things (dialogs, `.panel--float`) get `--shadow`. The sidebar on phones and the toolbar get the 82 % blur header. No other blur, glow or gradient.

## 1.8 Motion

Color and background fades only: 160 ms, `cubic-bezier(.25,.46,.45,.94)`. Nothing moves on scroll. No pulsing dots. `prefers-reduced-motion: reduce` removes every transition.

## 1.9 Icons

`static/icons.svg`, 16 px, 1.5 px stroke, `currentColor`. Use `<svg class="icon"><use href="/static/icons.svg#i-monitor"/></svg>`. Icons: monitor, heartbeat, incident, ticket, settings, bell, key, globe, users, plus, copy, chevron. Add a new symbol in the same style instead of pulling an icon library.
