# 4. Tech Stack

Every row was confirmed in the design grill on 2026-10-16.

## 4.1 Runtime and language

| Component | Technology | Justification |
| --- | --- | --- |
| Language | Rust 1.97, edition 2024 | One static binary. No runtime to install. |
| Async runtime | tokio 1.53 | Required by axum, reqwest, lettre and turso. |

## 4.2 Backend

| Component | Technology | Justification |
| --- | --- | --- |
| HTTP framework | axum 0.8 | Maintained by the tokio team. Most used Rust web framework. |
| Cookies | axum-extra 0.12 (`cookie` feature) | Session cookie handling. |
| Forms and JSON | serde 1, serde_json 1 | Standard. |
| Embedded files | rust-embed 8 | The built React app (`web/dist`), the theme script, the favicon and the fonts live inside the binary. |
| HTTP client | reqwest 0.13, `rustls` | Probes and webhooks. No OpenSSL. |
| Email | lettre 0.11, `tokio1-rustls` | SMTP. |
| Password hash | argon2 0.6 | Default Argon2id parameters. |
| Random tokens | rand 0.9 | Tokens, offsets. |
| Hashing | sha2 0.10 | API key, invite, session and ref ID hashes. |
| CLI | clap 4 (`derive`) | `serve`, `migrate`, `setup`, `service`, `whereis`, `admin` ([api-specs 10](../api-specs/10-cli.md)). |
| Logs | tracing 0.1, tracing-subscriber 0.3 | JSON in prod, pretty in dev. |
| Errors | thiserror 2, anyhow 1 | `thiserror` in modules, `anyhow` in `main`. |

## 4.3 Frontend

| Component | Technology | Justification |
| --- | --- | --- |
| UI | React 19, TypeScript, react-router 7 | One single-page app for every page, signed-in and public. Decision: [react-ui-migration](../decisions/2026-10-17-react-ui-migration.md). |
| Components | shadcn/ui (new-york, Radix), Phosphor icons, sonner | Copied as source into `web/src/components/ui` and restyled. See [design system](../design-system/). |
| Styling | Tailwind 4 with the tokens in `web/src/index.css` | No hex values in components. |
| Build | Bun (`web/build.ts`) | Hashed files under `web/dist/assets/`; `index.html` names them. No dev server in the e2e suite. |
| Charts | SVG drawn by React from server geometry (`checks::chart`) | No chart library; no unbounded rows reach the page. |

## 4.4 Datastore

| Component | Technology | Justification |
| --- | --- | --- |
| Database | turso 0.8 (SQLite file format) | User choice. Standard SQLite features only. |
| Migrations | `platform::migrate`, numbered `.sql` files embedded with `include_str!` | No sqlx. See [06 section 6.6](06-data-model.md#66-migrations-and-seeding). |

## 4.5 Testing and CI

| Component | Technology | Justification |
| --- | --- | --- |
| Unit and integration | `cargo test` | Tests use a temp-file database (`tempfile`) and fake adapters. nextest saves seconds at this size, so it is not used. |
| E2E | Playwright 1.64 on Bun, system Chrome | Drives the real debug binary on port 18080. Resets state through `/admin/reset-state`. |
| Lint and format | clippy (`-D warnings`), rustfmt | Pre-commit and CI. |
| Supply chain | cargo-deny | Licenses and advisories. |
| Task runner | `Makefile` | `check-fast`, `check` (local gate), `release-check` (adds the LTO build), `e2e`, `e2e-full`, `release`. 4 rustc jobs at `nice 10` by default. |
| Hooks | `.githooks/pre-commit` | `cargo fmt --check` and clippy when Rust files are staged. `make setup` enables it. |
| CI | GitHub Actions, `.github/workflows/ci.yml` | Actions pinned by commit SHA. `check` builds once (`make check`) and uploads the debug binary; `deny` runs beside it; e2e runs one job per shard (`e2e/lib/shards.ts`) on that binary and skips a shard that passed on the same inputs. Docs-only pull requests run the fast gate alone. |
| Release | `.github/workflows/release.yml` | On a `v*` tag push only: musl build, boot check, tar.gz with the systemd files, checksums, provenance attestation, draft GitHub release. |

## 4.6 Deployment

| Component | Technology | Justification |
| --- | --- | --- |
| Binary | `x86_64-unknown-linux-musl` static build | Copy one file to the VM. |
| Process | systemd unit | Restart on failure, journald logs. |

## 4.7 What we do not use

| Tool | Reason |
| --- | --- |
| sqlx | No Turso support. Third-party `sqlx-turso` is alpha. |
| ORMs (Diesel, SeaORM) | Plain SQL is enough for 15 tables. |
| Leptos, Dioxus | WASM UI frameworks. React has the shadcn components and the Beaverflow base (4.3). |
| Pico CSS | Archived. Tailwind 4 with Monita's tokens replaces hand-written CSS. |
| A JS chart library | The server lays out chart geometry; React draws SVG. |
| apalis, tokio-cron-scheduler | One DB-driven loop covers every job. See [13](13-check-scheduler.md). |
| axum-login, tower-sessions | One sessions table and one extractor cover it. |
| governor, tower_governor | A fixed-window counter is about 30 lines. |
| Redis, a queue broker | The outbox table is the queue. |

## 4.8 Version pinning

- `Cargo.lock` is committed. `Cargo.toml` uses caret versions at the minor level shown above.
- `rust-toolchain.toml` pins the compiler.
- Dependabot opens update PRs. CI must pass before merge.
