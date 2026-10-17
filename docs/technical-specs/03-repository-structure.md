# 3. Repository Structure

One Cargo crate. No workspace.

```text
monita/
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml        pins Rust 1.97
├── deny.toml                  cargo-deny: licenses and advisories
├── migrations/                numbered .sql files, built into the binary
│   └── 0001_init.sql
├── seeds/                     dev.sql, qa.sql and demo.sql, loaded by reset-state
├── src/
│   ├── lib.rs                 module tree; integration tests link against it
│   ├── main.rs                calls cli::run
│   ├── cli/                   mod.rs (commands, env file loading), setup.rs, service.rs, envfile.rs
│   ├── app.rs                 AppState, router assembly, background loop start
│   ├── platform/              config, db, migrate, ui (JSON envelope), spa (serves web/dist), page (PageError),
│   │                          assets, headers, health, rate_limit, limits, settings, jobs, backup, time,
│   │                          token, reset_state
│   ├── accounts/              mod.rs (interface), auth.rs (cookie, extractors), api.rs, admin_api.rs, store.rs
│   ├── projects/              mod.rs, allowlist.rs, icon.rs, profile.rs, overview.rs, api.rs, settings_api.rs,
│   │                          routes.rs (the icon), store.rs
│   ├── checks/                mod.rs, probe.rs, heartbeat.rs, cause.rs, scheduler.rs, retention.rs, insight.rs,
│   │                          overview.rs, series.rs, chart/, ping_routes.rs, api/{mod, monitors, heartbeats, views},
│   │                          store/{monitors, heartbeats, rollup}
│   ├── incidents/             mod.rs, engine.rs (pure), history.rs, manual.rs, availability.rs, api.rs,
│   │                          store/{mod, reads}
│   ├── notify/                mod.rs (send), delivery.rs (outbox loop), mailer.rs, digest.rs, api.rs, store.rs
│   ├── status/                mod.rs (public wire types), routes.rs (public JSON)
│   └── tickets/               mod.rs, store.rs, routes/{mod, api (/api/v1), public, board}
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
