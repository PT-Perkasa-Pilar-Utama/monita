# 3. Repository Structure

One Cargo crate. No workspace.

```text
monita/
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml        pins Rust 1.97
├── deny.toml                  cargo-deny: licenses and advisories
├── migrations/                numbered .sql files, built into the binary
│   └── 0001_init.sql ... 0022_supervisor_role.sql   append only
├── seeds/                     dev.sql, qa.sql and demo.sql, loaded by reset-state
├── src/
│   ├── lib.rs                 module tree; integration tests link against it
│   ├── main.rs                calls cli::run
│   ├── cli/                   mod.rs (commands, env file loading), setup.rs, service.rs, envfile.rs
│   ├── app.rs                 AppState, router assembly, background loop start
│   ├── platform/              config, db, migrate, ui (JSON envelope), spa (serves web/dist), page (PageError),
│   │                          assets, headers, health, rate_limit, limits, caps, settings, mail_api, jobs, backup,
│   │                          time, token, reset_state
│   ├── accounts/              mod.rs (interface), auth.rs (cookie, extractors), admin.rs (roles, disable), password.rs,
│   │                          api.rs, admin_api.rs, store.rs
│   ├── projects/              mod.rs, allowlist.rs, icon.rs, profile.rs, overview.rs, pulse.rs, api.rs,
│   │                          settings_api.rs, routes.rs (the icon), secrets/{mod, api, store}, store/{mod, keys}
│   ├── checks/                mod.rs, input.rs, probe.rs, heartbeat.rs, cause.rs, scheduler.rs, retention.rs,
│   │                          insight.rs, overview.rs, series.rs, chart/, ping_routes.rs,
│   │                          api/{mod, monitors, heartbeats, views},
│   │                          store/{monitors, heartbeats, results, sources, rollup, charts}
│   ├── incidents/             mod.rs, engine.rs (pure), updates.rs, history.rs, manual.rs, availability.rs, api.rs,
│   │                          store/{mod, reads, across}
│   ├── notify/                mod.rs (send), delivery.rs (outbox loop), mailer.rs, digest.rs, preview.rs,
│   │                          tickets.rs (ticket cards), cards/, telegram/, api.rs, store.rs
│   ├── status/                mod.rs (public wire types), routes.rs (public JSON), months.rs, uptime_calendar.rs
│   ├── tickets/               mod.rs, routes/{mod, api (/api/v1), public, board}, store/{mod, across}
│   ├── changelog/             mod.rs, api.rs (team and public routes), store.rs
│   └── home/                  mod.rs, api.rs (Incidents and Tickets across projects)
├── web/                       React app: src/{pages, layouts, components, lib}, build.ts; `make web` builds dist/
├── static/                    theme.js, favicon.svg, fonts; embedded by rust-embed
├── tests/                     Rust integration tests (temp-file DB, fake adapters)
├── e2e/                       Playwright on Bun: package.json, playwright.config.ts, specs/, demo/ (static demo build)
├── site/                      build.ts, landing page, site.css, docs.js: the GitHub Pages site
├── scripts/                   smoke-cli.sh: `setup` and `service` as root on CI
├── deploy/                    monita.service (systemd), monita.env.example
├── .githooks/                 pre-commit
├── Makefile                   the commands in 04 section 4.5
└── docs/
```

| Folder | Rule |
| --- | --- |
| `src/<module>/mod.rs` | The module interface. Only `pub` items here are visible to other modules. |
| `src/<module>/store.rs` | SQL for that module's tables only. |
| `src/incidents/engine.rs` | Pure functions. No I/O. Tested without a database. |
| `src/<module>/api.rs` | The module's `/api/ui/` JSON. Parses, calls the interface, answers. |
| `web/src/components/ui/` | shadcn components restyled to the [design system](../design-system/). Pages do not write their own buttons or tables. |
| `web/src/lib/` | The API client, wire types, and pure helpers, each with a `bun test`. |
| `migrations/` | Append only. A merged migration never changes. |
