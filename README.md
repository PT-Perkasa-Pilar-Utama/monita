# Monita

Monita is a self-hosted uptime and incident service for the projects of one IT department. It is a small version of Better Stack. Monita checks project URLs, receives heartbeat pings from cron jobs, opens and closes incidents, sends alerts, publishes a status page per project, and collects bug tickets from project users.

It ships as one Rust binary with one SQLite-format database file. The motto: easy to integrate, simple, lightweight, fast, reliable.

## Status

v1 is in build. Features land sprint by sprint. Seeded dev accounts are listed in [seeds/README.md](seeds/README.md).

| Sprint | Delivers | State |
| --- | --- | --- |
| 1. Checks run | Accounts and invites, projects, API keys, HTTP monitors, heartbeats | Done |
| 2. Incidents and alerts | Automatic incidents, status updates, email and webhook alerts | Planned |
| 3. Public status | Status page, public JSON, rate limits, retention, backup | Planned |
| 4. Tickets | Ref IDs, bug tickets, kanban, public board | Planned |

Stories and acceptance criteria: [docs/business/](docs/business/).

## Stack

| Layer | Technology |
| --- | --- |
| Language | Rust 1.97 |
| HTTP | axum 0.8, tokio |
| UI | Server-rendered askama templates, htmx 2, one CSS file ([design system](docs/design-system/)) |
| Database | turso 0.8 (SQLite file format) |
| Tests | `cargo test`, Playwright on Bun |

## Quickstart

Prerequisites: Rust (the toolchain in `rust-toolchain.toml` installs itself through rustup), Bun 1.4 and Google Chrome for the browser tests.

```sh
make setup          # enable the git hooks, install the e2e packages
make run            # serve on http://127.0.0.1:8080 with ./data/monita.db
```

Check that it runs:

```sh
curl -s http://127.0.0.1:8080/healthz
# {"status":"ok","db":"ok","scheduler_lag_s":0,"outbox_pending":0,"version":"0.1.0"}
```

Run the gate before you hand off a change:

```sh
make check          # format, clippy, unit and integration tests
make e2e            # browser tests against the debug binary on port 18080
```

Configuration comes from `MONITA_*` environment variables: [docs/technical-specs/11-environment-configuration.md](docs/technical-specs/11-environment-configuration.md). A production example is in [deploy/monita.env.example](deploy/monita.env.example), and the systemd unit is [deploy/monita.service](deploy/monita.service).

## Repository map

```text
src/            Rust crate: one folder per module (accounts, projects, checks, incidents,
                notify, status, tickets) plus platform (config, db, migrations, rendering)
templates/      askama templates: layout/, ui/ macros, one folder per module
static/         app.css, icons, fonts, htmx; embedded in the binary
migrations/     numbered SQL files, applied at start
seeds/          dev and QA data for /admin/reset-state (never in prod)
tests/          Rust integration tests
e2e/            Playwright suite and its CI shard table
deploy/         systemd unit and environment example
docs/           decisions, stories, specs, API contracts, design system
```

## Docs

| Read | For |
| --- | --- |
| [CLAUDE.md](CLAUDE.md) | Rules for anyone changing the code, human or agent |
| [docs/decisions/](docs/decisions/) | Why Monita is shaped this way |
| [docs/technical-specs/](docs/technical-specs/) | Architecture, modules, data model, security |
| [docs/api-specs/](docs/api-specs/) | The HTTP contracts projects integrate against, and what is built |
| [docs/design-system/](docs/design-system/) | UI tokens, components, page recipes |

## Releases

Push a `v*` tag that matches the version in `Cargo.toml`. [release.yml](.github/workflows/release.yml) builds the static Linux binary and opens a draft GitHub release.
