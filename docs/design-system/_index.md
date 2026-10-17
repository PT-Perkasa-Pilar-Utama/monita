# Monita Design System

**Version:** 1.0  
**Date:** 2026-10-16  
**Author:** snowfluke  
**Status:** Approved for build  

Flavor: Lunear ([source](source-lunear.md)), adapted for an ops tool. Code: `static/app.css`, `static/icons.svg`, `static/fonts/`, `templates/ui/`. Live preview: https://claude.ai/artifact/QPMdi64uG7oYZyhuSkBHjn

## Table of Contents

1. [Foundations](01-foundations.md): schemes, color tokens, type, space, shape, motion
2. [Components](02-components.md): every class and its askama macro
3. [Patterns](03-patterns.md): shells, page recipes, copy rules, do and don't

## Deviations from Lunear

| Lunear | Monita | Why |
| --- | --- | --- |
| Inter and Geist Mono from Google Fonts | Embedded woff2 in the binary (78 KB) | The data center may block CDNs. |
| Dark only | Dark first, plus a Daylight scheme from the OS setting | The public status page serves everyone. |
| Marketing layout (160 px sections, hero glow) | App layout (a top bar and tab bar, 14 px UI), sized like the docs | Monita is the app, not a page about it. |
| Green, red and yellow inside mocks only | Same colors carry monitor state and incident labels | "Colour is data" already says this. |
