# Brand

Monita's logo, its mascot Moni, the colours and the type, as files to download and rules to use them by.

## Logo

The mark is a pulse in a ring: a heartbeat that Monita keeps an eye on. The logo sets the mark beside the name, as the app's header does.

<!-- brand:logo -->

- Use the file made for the background: `logo.svg` on light, `logo-on-dark.svg` on dark. Use `mark-white.svg` on a photo or a strong colour.
- Keep clear space of half the mark's height on every side.
- Show the mark at 16 px or more, and the logo at 96 px wide or more.
- Do not recolour, stretch, rotate, outline or shadow the mark. Do not set the name in another typeface.
- The SVG logos draw the name with Inter. On a computer without Inter, use the PNG.

## Moni

Moni is Monita's mascot, built from the mark: the ring is the body, the pulse is the mouth, and the antenna listens. Moni appears on the sign-in page, under every primitive's pages in the app, on the landing page, and in the README.

<!-- brand:moni -->

- Moni looks the same in both themes. Never invert or recolour Moni for dark mode.
- Moni wears the brand magenta. Nothing else in a drawing does, except the lines that show what Monita sends.
- A drawing shows states in green and grey only. Never red or yellow: a picture that is always on screen must not look like an alarm.
- Flows show their direction: magenta for what Monita sends, green for an answer.
- Keep Moni's proportions. Do not crop the antenna or the feet, except in the peeking pose.
- Motion stays small: a blink, a beat, a bounce. It stops when the reader asks for reduced motion.

### Scenes

Each primitive has a scene, which the app draws at the foot of its pages and the landing page shows beside each part. The files use the light colours. A page that inlines one can recolour it for its theme with the `--art-*` CSS variables, as the landing page does.

<!-- brand:scenes -->

## Colours

Magenta is the brand. It marks the mark, the main button, links, focus and the chart line. It never marks a state.

| Role | Light | Dark | Use |
| --- | --- | --- | --- |
| Brand | <span class="swatch" style="background:#c4258f"></span> `#c4258f` | <span class="swatch" style="background:#f25cbf"></span> `#f25cbf` | Mark, main button, links, focus |
| Up | <span class="swatch" style="background:#23784e"></span> `#23784e` | <span class="swatch" style="background:#4cb782"></span> `#4cb782` | A system that works |
| Down | <span class="swatch" style="background:#d24545"></span> `#d24545` | <span class="swatch" style="background:#eb5757"></span> `#eb5757` | A system that is down |
| Issue | <span class="swatch" style="background:#8a6300"></span> `#8a6300` | <span class="swatch" style="background:#f2c94c"></span> `#f2c94c` | A problem that is not an outage |
| Page | <span class="swatch" style="background:#f7f8f8"></span> `#f7f8f8` | <span class="swatch" style="background:#08090a"></span> `#08090a` | Background |
| Card | <span class="swatch" style="background:#ffffff"></span> `#ffffff` | <span class="swatch" style="background:#0f1011"></span> `#0f1011` | Cards and the header |
| Ink | <span class="swatch" style="background:#16171a"></span> `#16171a` | <span class="swatch" style="background:#f7f8f8"></span> `#f7f8f8` | Text |

Moni has its own colours, the same in both themes:

| Part | Colour |
| --- | --- |
| Body | <span class="swatch" style="background:#fbe4f2"></span> `#fbe4f2` |
| Line, feet and antenna | <span class="swatch" style="background:#c4258f"></span> `#c4258f` |
| Eyes | <span class="swatch" style="background:#1e1a1d"></span> `#1e1a1d` |
| Cheeks | <span class="swatch" style="background:#f2a7d4"></span> `#f2a7d4` |

The app's source of truth is `web/src/index.css`.

## Type

| Use | Typeface |
| --- | --- |
| Text and headings | Inter, variable weight |
| Code, numbers in tables, addresses | Geist Mono |

Both ship with Monita in `static/fonts/`.

## Words

The tagline is **Site reliability for every project your team ships.** Write in plain words and whole sentences. Say what happened and what to do next. Never use a dot or a dash to join two ideas.

## Make the files again

The files come from the app's own drawing code in `web/src/components/brand/`, so they always match the app. After a change there, run:

```sh
make brand
```

It writes the SVGs, then the PNGs, into `docs/assets/brand/`.
