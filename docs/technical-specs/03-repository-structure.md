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
├── seeds/                     dev.sql and qa.sql, loaded by reset-state
├── src/
│   ├── lib.rs                 module tree; integration tests link against it
│   ├── main.rs                CLI: `serve`, `migrate`, `admin create`
│   ├── app.rs                 AppState, router assembly, background loop start
│   ├── platform/              config, db, migrate, render, assets, health, rate_limit,
│   │                          settings, backup, peer, reset_state
│   ├── accounts/              mod.rs (interface), routes.rs, store.rs
│   ├── projects/              mod.rs, routes.rs, store.rs
│   ├── checks/                mod.rs, probe.rs, scheduler.rs, retention.rs, routes.rs, store.rs
│   ├── incidents/             mod.rs, engine.rs (pure), store.rs, routes.rs
│   ├── notify/                mod.rs, outbox.rs, mailer.rs, webhook.rs, digest.rs, store.rs
│   ├── status/                routes.rs
│   └── tickets/               mod.rs, routes.rs, store.rs
├── templates/                 askama templates, one folder per module, plus layout/ and ui/
├── static/                    app.css, htmx.min.js (2.0.11), icons.svg; embedded by rust-embed
├── tests/                     Rust integration tests (temp-file DB, fake adapters)
├── e2e/                       Playwright on Bun: package.json, playwright.config.ts, specs/
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
| `templates/ui/` | Shared macros from the [design system](../design-system/). Modules do not write their own buttons or tables. |
| `migrations/` | Append only. A merged migration never changes. |
