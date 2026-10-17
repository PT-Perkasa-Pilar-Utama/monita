# React UI migration: decisions

Date: 2026-10-17

The user asked to move every screen from askama templates to React with shadcn/ui and Tailwind, using Beaverflow's frontend as the base and Monita's own look.

| # | Question | Decision | Why |
|---|----------|----------|-----|
| Q1 | What migrates? | Every page the binary serves: the app, sign-in and invite, the public status pages, the bug report form, the public ticket board and the error page. | One UI stack. |
| Q2 | What stays out? | `site/` (landing page and docs) and `e2e/demo`. Their screenshots are taken again at the end. | They are a separate static site, not pages the binary serves. |
| Q3 | Where does the frontend live? | `web/`, React 19, TypeScript, Tailwind 4, shadcn/ui (new-york, Radix, Phosphor icons). Bun builds it to `web/dist`, which rust-embed puts in the binary. | Same toolchain as Beaverflow and the e2e suite. One binary stays one binary. |
| Q4 | How does the page get data? | Session-cookie JSON under `/api/ui/...`, one `*_api.rs` per module, calling the same module interfaces. Errors use the api-specs 01 envelope plus a field-keyed `errors` map for forms. | Handlers stay thin; module rules do not move. |
| Q5 | CSRF | The Origin check now exempts only `/api/v1/`, `/ping/` and reset-state. `/api/ui/` mutations need Monita's Origin, which every browser `fetch` sends. | `/api/ui/` uses the session cookie. |
| Q6 | CSP | `style-src 'self' 'unsafe-inline'`; `script-src 'self'` stays strict. | Radix positions popovers and dialogs with inline styles. Scripts are still only Monita's own files. |
| Q7 | Charts | The API returns the same bounded buckets from `checks::chart`; a small React SVG component draws them. No chart library. | The server still does the aggregation; nothing unbounded reaches the page. |
| Q8 | No-JavaScript fallback | Dropped. | A React page needs its script. |
| Q9 | Done | An adversarial review finds nothing on its checklist, or 15 review rounds pass. | The user's goal. |

## Review checklist

A finding is one of:

- a `derive(Template)` or `askama` reference left in `src/` or `Cargo.toml`;
- a file left in `templates/`;
- an api-specs route row that still describes an HTML form;
- an integration test that asserts on HTML markup;
- an e2e spec that fails or is skipped;
- a SEC-14 or SEC-21 leak in a public React view;
- a CLAUDE.md or doc row that contradicts the code;
- `make check`, `make e2e` or `make release-check` failing;
- a doc screenshot of the old UI.

Cosmetic preferences are not findings.
