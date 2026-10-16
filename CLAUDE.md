# Monita

Self-hosted uptime, incident, status-page and bug-ticket service for every project in the IT department. One Rust binary, one SQLite-format file.

Gate before marking any task complete: `make check`. If the change touches a page or an API, also run `make e2e`.

## Source of truth

| Concern | Doc |
| --- | --- |
| Scope decisions and terms | `docs/decisions/2026-10-16-monita-v1-scope.md` |
| Stories, sprints, Gherkin AC | `docs/business/` |
| Architecture, modules, data model, security | `docs/technical-specs/` (12 incident engine, 13 scheduler) |
| Wire contracts and the operation tracker | `docs/api-specs/` |
| UI tokens, components, page recipes | `docs/design-system/` |

This file restates rules tersely. The docs win on any conflict; fix this file when they do.

## Project overview

| Layer | Technology |
| --- | --- |
| Language | Rust 1.97.1, edition 2024 (`rust-toolchain.toml`) |
| HTTP | axum 0.8, tokio 1.53 |
| HTML | askama 0.16 templates in `templates/`, htmx 2.0.11 vendored |
| CSS | `static/app.css` (Lunear flavor), `static/icons.svg`, embedded fonts. All embedded by rust-embed. |
| Database | turso 0.8.3, standard SQLite features only |
| Outbound (later sprints) | reqwest 0.13 rustls, lettre 0.11 |
| Tests | `cargo test` (temp-file DB), Playwright 1.63 on Bun with system Chrome |

```text
src/lib.rs            module tree        src/main.rs   CLI: serve, migrate
src/app.rs            AppState, router, build(), serve()
src/platform/         config, db, migrate, render, assets, headers, health, reset_state
src/<module>/         mod.rs (interface) routes.rs store.rs + logic files
templates/layout/     base, app (sidebar), public, bare     templates/ui/ macros.html, mark.html
migrations/           0001_init.sql, append only            seeds/ dev.sql, qa.sql
tests/                integration, tests/common::TestApp    e2e/ specs, lib, global-setup.ts
```

## Boundaries

| Module | Owns tables | Interface others call |
| --- | --- | --- |
| accounts | users, invites, sessions | `CurrentUser` extractor, `require_role` |
| projects | projects, api_keys | `ApiProject` extractor, `owner_of`, `can_manage` |
| checks | monitors, heartbeats, check_results, pings, daily_summaries | `probe`, `record_ping`, `scheduler::spawn` |
| incidents | incidents, incident_items, incident_sources, status_updates | `observe` (the only door for check results), `open_manual`, `post_update`, `states_for`, `source_deleted` |
| notify | alert_routes, outbox | `send(Event)`: inserts outbox rows, never blocks |
| status | none | public page and JSON |
| tickets | ref_ids, tickets | `issue_ref_id`, `submit` |
| platform | settings, schema_migrations | `Db`, `render`, `now`, config |

Roles: `super_admin`, `tl`. A TL manages only projects they own; a non-owner gets 404. Matrix: technical-specs 09 section 9.3.

## Work model

Two builders (the user and Claude), 2 days, sprints 1 to 4 in `docs/business/sprint-breakdown.md`. Build one story end to end: migration if needed, store, logic, route, template, tests. Then set its rows in `docs/api-specs/_index.md` to `OK`. Sprint 4 (tickets) is the first cut if time runs short.

## Architecture laws

- A module touches only its own tables. Cross-module calls go through `pub` items in `mod.rs`. Never import another module's `store` or `engine`.
- Handlers in `routes.rs` parse, call, render. No SQL and no business rules in handlers.
- SQL lives in `store.rs`. Rules live in `mod.rs` or a named logic file (`engine.rs`, `probe.rs`).
- Every write goes through `Db::write` (one writer connection, `BEGIN IMMEDIATE`). Reads use `Db::read()`. Never open a writing connection elsewhere.
- No transaction stays open across a network call.
- Templates use `templates/ui/macros.html`. No inline `style=` (CSP blocks it). No color literal outside `:root` in `app.css`.
- File cap: 400 lines per `.rs`, 200 per template. Past it, turn the file into a folder module and keep `mod.rs` as the interface.

## Runtime do and don't

| Do | Don't |
| --- | --- |
| `make` targets, `cargo` | nextest, cargo-watch, bacon (not installed, not needed) |
| `conn.query(...)` for any statement that returns rows | `conn.execute(...)` on a SELECT or a row-returning PRAGMA: turso errors "unexpected row" |
| `{% call ui::x(...) %}{% endcall %}` | `{% call %}` without `{% endcall %}`: askama 0.16 fails to compile |
| `Db::write(async \|c\| { ... })` | Many connections writing: turso returns busy instead of waiting |
| `bun` in `e2e/` | npm, npx, `playwright install` (system Chrome is used) |
| `platform::now()` for Unix seconds | Local time in storage |

## Verification gate

```sh
make check      # cargo fmt --check, clippy -D warnings, cargo test --lib, cargo test --tests
make e2e        # when a page or API changed: debug build, Playwright, server stopped at the end
```

`make release-check` adds the 5-minute LTO build. CI (`.github/workflows/ci.yml`, testate's pattern) runs `make check` once, cargo-deny beside it, then the e2e shards from `e2e/lib/shards.ts` on the debug binary that `check` built. A shard skips itself only when a pass on the same inputs is recorded in the cache. The required check `browser end-to-end` passes a skipped e2e only when nothing but docs changed and `check` passed. `release.yml` runs only on a `v*` tag push and opens a draft release.

A new e2e spec file must match a project in `e2e/lib/shards.ts`; `bun test lib` in `e2e/` fails otherwise.

## Code discovery protocol

1. Read the story's AC in `docs/business/acceptance-criteria-breakdown/` and its contract in `docs/api-specs/`.
2. Read the module's `mod.rs` and a sibling module that already does something similar.
3. Reuse `platform` helpers (`query_i64`, `render`, `now`) and `ui` macros before writing new ones.

## Naming

| Element | Pattern | Example |
| --- | --- | --- |
| Table | plural snake_case | `check_results` |
| Time column | `_at`, Unix seconds INTEGER | `next_check_at` |
| Enum value in DB | lowercase snake | `in_progress`, `super_admin` |
| Route | kebab path, slug not id for projects | `/projects/{slug}/monitors/{id}` |
| JSON API | `/api/v1/...`, snake_case fields | `single_use` |
| Template | `templates/<module>/<page>.html` | `templates/checks/monitors.html` |
| Test | behavior sentence | `reset_state_does_not_exist_in_prod` |
| e2e test | `@US-xx` tag in the title | `@US-09 TL creates a monitor` |

## Types, errors, tests, database

- Validate input at the boundary (form, JSON, path). Return 422 with the form re-rendered, or the JSON error envelope in api-specs 01 section 1.2.
- `anyhow` in app code paths, `thiserror` for errors a caller branches on. Never swallow an error: log it with `tracing` and return a status.
- Every rule with a branch gets a test at the module interface. The incident engine (`step`) is pure and fully branch-tested.
- Migrations are append only. A new change is `migrations/000N_<what>.sql` plus one line in `platform/migrate.rs`.
- Seeds hold statements that return no rows.

## Git

- Work on `main`. Commit after every finished piece of work, without asking: one commit per intent (story, fix, docs), after the gate passes. Format `type(scope): subject`, for example `feat(checks): run monitors on the scheduler`.
- No co-author or "Generated with" trailers (`.claude/settings.json` turns them off).
- Never push. The user pushes.

## Absolute prohibitions

| Violation | Why |
| --- | --- |
| `/admin/reset-state` reachable when `MONITA_ENV=prod` | SEC-17: the route must not be registered |
| Raw API keys, session tokens, invite tokens or ref IDs stored or logged | Only SHA-256 hashes are stored (SEC-07, SEC-20) |
| Ticket description or reporter shown on a public page | SEC-14 |
| An update's author, or Monita's cause text, on a status page | SEC-21 |
| A Triage ticket on the public board | Decision Q11 |
| Checks waiting on SMTP or webhooks | Alerts go through the outbox (spec 05 section 5.5) |
| Editing a merged migration | Breaks every existing database |
| A new dependency for something a few lines can do | Footprint is the motto |

## Context recovery checklist

- [ ] An incident opens after 2 failed checks in a row; a missed heartbeat or `/ping/{token}/fail` opens it at once; the first pass closes an auto incident; manual incidents close only on a TL "Resolved" update.
- [ ] Status update labels: investigating, identified, monitoring, resolved.
- [ ] Ticket stages: triage, backlog, in_progress, done. Done needs close_reason fixed, wont_fix or duplicate. Triage is private.
- [ ] Severity: critical (email + webhook by default), warning (email only).
- [ ] Monitor intervals: 30, 60, 180, 300 s. Default timeout 10 s, expected `2xx`.
- [ ] Ref IDs: `POST /api/v1/ref-ids`, project from the API key, `single_use` flag. Caps: 5 per ref ID per day, 100 per project per day.
- [ ] Rate limits: 60/min per IP (public, ticket form, login), 120/min per API key, 1/s per heartbeat (extra pings return 429 and still count); `/fail` has its own 1/s bucket `pingfail:{id}`. Request bodies are capped at 128 KiB (413).
- [ ] API key format `mk_` + 26 base32 chars; heartbeat tokens stored raw; everything else hashed.
- [ ] Scheduler: one 1 s loop; claim by setting `next_check_at` before probing; 50 concurrent checks. Each tick holds one shared read guard on `AppState.gate`; reset-state takes the write guard.
- [ ] `incidents::observe` and `notify::send` run inside the same `Db::write` as the check or ping that caused them. The outbox claims rows with a 120 s lease.
- [ ] Dev machine has 8 GiB RAM: 4 cargo jobs, e2e at most 2 workers.

## Quick reference

```sh
make setup                      # git hooks + e2e deps
make run                        # dev server on 127.0.0.1:8080, ./data/monita.db
make check-fast                 # fmt, clippy, unit tests
make check                      # the gate
make e2e                        # Playwright smoke and story specs, port 18080
cargo run -- migrate            # apply migrations and exit
curl -s localhost:8080/healthz  # health
curl -s -X POST localhost:8080/admin/reset-state -H "x-reset-token: $MONITA_RESET_TOKEN" \
  -H 'content-type: application/json' -d '{"seed":"dev"}'   # dev/test only
```

# Accuracy
State what you can verify. Mark everything else.
State a confidence level (high, moderate, low, unknown) only when it changes what I should do.
Say "I don't know" and stop. Do not fill gaps with plausible detail.
Cite sources for figures, dates, quotes, and names.
Search when a claim is current, contested, or after your cutoff.
Show the arithmetic for any number you produce.
Form your own estimate before you use mine. Compare both.

# Directness
Tell me when I am wrong. Do it in the first sentence.
Start with the answer. Skip praise and preamble.
Deliver bad news plain.
Hold your position when I push back. Change it for new evidence or a better argument.
Keep caveats that change my decision. Cut the rest.
Do not soften, hedge, or moralize unless I ask.

# Reasoning
State the strongest objection to your own conclusion. Then answer it.
Separate what you know from what you infer.

# Format
Match length to the question.
Write prose. Use lists for real lists.
Follow ASD-STE100: one instruction per sentence, active voice, simple tenses, 20 words maximum.
Use the /ste100 skill for manuals and specifications.

# Ambiguity
Ask one question when the request is unclear and a wrong answer is costly.
Otherwise state your assumption and proceed.

# Code quality
Write for the next person who opens the file. Write a comment only for a constraint the code cannot show.
Reuse an existing function before you write a new one.
Keep each function to one job.
Type every interface, API contract, and data shape.
Handle errors at the boundary. Do not swallow them.
Name the technical debt you create. Say what would clear it.
State the trade-off when you choose speed over structure.
Skip this rigor for throwaway scripts. Tell me when you skip it.

# Continuity
Read the existing code before you extend it. Match its patterns.
Keep names, structure, and conventions stable across the session.
Edit the existing file. Do not regenerate it from scratch.

# State machines
Define an explicit state machine for anything with a status.
List every state. List every legal transition.
Name the actor and the guard condition for each transition.
Reject any transition that no rule allows.
Show the machine as a table before you write the code.
Name the terminal states.

# Processes
Clean up every process you start.
- Before you start a server, browser, watcher, emulator, or test runner, check for one that already runs. Reuse it.
- Track each long-running process you start: its PID, its port, and how to stop it.
- Prefer commands that exit when they finish. Avoid watch mode and background processes unless the task needs them.
- Never run a broad kill such as `pkill node`. Kill only the processes you started. Ask before you stop any other process.
- When the machine is slow, check process age, CPU, memory, and parent processes. Clean up your own leftover processes before you start new ones.

# End-to-end tests
- Run only the specs the change touches. The full suite runs in CI.
- Use the project's local worker cap and one headless browser.
- Skip scenarios tagged heavy unless I ask for them.
- Stop every server and browser the run started, even when the run fails.

# Implementation
- Do not preserve backwards compatibility unless the docs say so.
- Choose the simplest implementation that fully meets the current requirements. Do not over-engineer.
- Reuse what the project, the standard library, or an installed dependency already has. Add a new dependency only when a few lines of code will not do.
- Make architectural decisions for the long term. When you take a stopgap, name its limit and what replaces it.
