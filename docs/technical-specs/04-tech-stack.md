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
| Templates | askama 0.16 | Compile-time templates. A broken template fails the build. Rendered by a 5-line helper in `platform::render`. |
| Static assets | rust-embed 8 | CSS, the copy-button script, fonts and icons live inside the binary. |
| HTTP client | reqwest 0.13, `rustls` | Probes and webhooks. No OpenSSL. |
| Email | lettre 0.11, `tokio1-rustls` | SMTP. |
| Password hash | argon2 0.6 | Default Argon2id parameters. |
| Random tokens | rand 0.9 | Tokens, offsets. |
| Hashing | sha2 0.10 | API key, invite, session and ref ID hashes. |
| CLI | clap 4 (`derive`) | `serve`, `admin create`. |
| Logs | tracing 0.1, tracing-subscriber 0.3 | JSON in prod, pretty in dev. |
| Errors | thiserror 2, anyhow 1 | `thiserror` in modules, `anyhow` in `main`. |

## 4.3 Frontend

| Component | Technology | Justification |
| --- | --- | --- |
| Rendering | Server-side HTML | No build step. |
| Interactivity | Plain HTML forms and one small script for copy buttons | htmx was vendored and removed unused on 2026-10-17 (footprint). Add it back if a page needs partial swaps. |
| Styling | Hand-written CSS with tokens | See [design system](../design-system/). Pico CSS is archived. |
| Charts | Inline SVG from templates | Uptime bars only. |

## 4.4 Datastore

| Component | Technology | Justification |
| --- | --- | --- |
| Database | turso 0.8 (SQLite file format) | User choice. Standard SQLite features only. |
| Migrations | `platform::migrate`, numbered `.sql` files embedded with `include_str!` | No sqlx. See [06 section 6.6](06-data-model.md#66-migrations-and-seeding). |
| Fallback | rusqlite (bundled) | Opens the same file if Turso fails. |

## 4.5 Testing and CI

| Component | Technology | Justification |
| --- | --- | --- |
| Unit and integration | `cargo test` | Tests use a temp-file database (`tempfile`) and fake adapters. nextest saves seconds at this size, so it is not used. |
| E2E | Playwright 1.63 on Bun, system Chrome | Drives the real debug binary on port 18080. Resets state through `/admin/reset-state`. |
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
| Leptos, Dioxus, React | A WASM or JS build for four pages. |
| Tailwind, Pico CSS | A build step, and Pico is archived. |
| apalis, tokio-cron-scheduler | One DB-driven loop covers every job. See [13](13-check-scheduler.md). |
| axum-login, tower-sessions | One sessions table and one extractor cover it. |
| governor, tower_governor | A fixed-window counter is about 30 lines. |
| Redis, a queue broker | The outbox table is the queue. |

## 4.8 Version pinning

- `Cargo.lock` is committed. `Cargo.toml` uses caret versions at the minor level shown above.
- `rust-toolchain.toml` pins the compiler.
- Dependabot opens update PRs. CI must pass before merge.
