# Monita Design System

**Version:** 2.0  
**Date:** 2026-10-19  
**Author:** snowfluke  
**Status:** Matches the code  

Flavor: Lunear ([source](source-lunear.md)), adapted for an ops tool. The UI is a React 19 app in `web/`. It uses Tailwind 4 and shadcn/ui (new-york style, Radix primitives, Phosphor icons). Components come from Beaverflow. Colors, type and shape are Monita's own.

## Code map

| Concern | Path |
| --- | --- |
| Tokens, fonts, dark mode | `web/src/index.css` |
| shadcn components, restyled | `web/src/components/ui/` |
| Monita components | `web/src/components/app/`, `list/`, `layout/`, `brand/` |
| Icons (the only import path) | `web/src/components/icons/index.ts` |
| Shells | `web/src/layouts/` |
| Pages | `web/src/pages/`, `pages/project/`, `pages/public/`, `pages/admin/` |
| Theme before paint | `static/theme.js` |
| Fonts | `static/fonts/` |

## Table of Contents

1. [Foundations](01-foundations.md): tokens, themes, type, shape, motion, icons, pointer, mascot
2. [Components](02-components.md): every component, its variants and its use, including tickets and illustrations
3. [Patterns](03-patterns.md): shells, navigation, page recipes, forms, copy rules, do and don't, footer illustrations, settings pages, read-only pages

## Deviations from Lunear

| Lunear | Monita | Why |
| --- | --- | --- |
| Inter and Geist Mono from Google Fonts | Woff2 files in the binary | The data center may block CDNs. |
| Dark only | Light and dark, with a switch | The public status page serves everyone. |
| Marketing layout (160 px sections, hero glow) | App layout: a top bar, a tab bar, 14 px text | Monita is the app, not a page about it. |
| Green, red and yellow inside mocks only | The same colors carry monitor state and incident labels | Color is data. |
