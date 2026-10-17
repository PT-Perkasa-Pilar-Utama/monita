# Monita

Monita is a self-hosted uptime and incident service for the projects of one IT department. It is a small version of Better Stack. Monita checks project URLs, receives heartbeat pings from cron jobs, opens and closes incidents, sends alerts, publishes a status page per project, and collects bug tickets from project users.

It ships as one Rust binary with one SQLite-format database file. The motto: easy to integrate, simple, lightweight, fast, reliable.

## What it does

| Feature | How it works |
| --- | --- |
| HTTP monitors | Monita requests a URL every 30 s to 5 min and checks the status code, an optional keyword, and the timeout. |
| Heartbeats | A cron job calls `/ping/{token}` after each run. A missed ping, or a call to `/ping/{token}/fail`, opens an incident. |
| Incidents | Two failed checks in a row open an incident. The first pass closes it. A tech lead can also open a manual incident and post updates: investigating, identified, monitoring, resolved. |
| Alerts | Email and webhook, per project and severity. Delivery goes through an outbox, so a slow SMTP server never delays a check. |
| Status page | Each project gets `/status/{slug}` and a public JSON at `/api/v1/public/status/{slug}`, with 90 days of uptime. |
| Bug tickets | The project's backend issues a ref ID with its API key. A user sends a ticket with that ref ID through the status page or the API. The tech lead moves it on a kanban; a public board shows the accepted ones. |

Two roles: a super admin invites tech leads, and each tech lead manages only the projects they own. User stories and acceptance criteria are in [docs/business/](docs/business/).

Integrate a cron job in one line:

```sh
if backup.sh; then curl -fsS -m 10 https://monita.example.com/ping/<token>
else curl -fsS -m 10 https://monita.example.com/ping/<token>/fail; fi
```

## Stack

| Layer | Technology |
| --- | --- |
| Language | Rust 1.97 |
| HTTP | axum 0.8, tokio |
| UI | Server-rendered askama templates and plain forms, one CSS file ([design system](docs/design-system/)) |
| Database | turso 0.8 (SQLite file format) |
| Tests | `cargo test`, Playwright on Bun |

## Quickstart

Prerequisites: Rust (the toolchain in `rust-toolchain.toml` installs itself through rustup), Bun 1.4 and Google Chrome for the browser tests.

```sh
make setup          # enable the git hooks, install the e2e packages
make run            # serve on http://127.0.0.1:8080 with ./data/monita.db
```

To load the demo project and the dev accounts in [seeds/README.md](seeds/README.md), start with a reset token and reset once:

```sh
MONITA_RESET_TOKEN=dev make run
curl -s -X POST http://127.0.0.1:8080/admin/reset-state -H 'x-reset-token: dev' \
  -H 'content-type: application/json' -d '{"seed":"dev"}'
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
static/         app.css, app.js, icons, fonts; embedded in the binary
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

## Run it in production

Monita runs as one systemd service on a Linux VM. These steps assume Ubuntu or Debian and root access.

1. Install the binary from a release (or build it with `make release`):

   ```sh
   tar -xzf monita_v0.1.0_linux_amd64.tar.gz
   install -m 0755 monita /usr/local/bin/monita
   useradd --system --home /var/lib/monita --shell /usr/sbin/nologin monita
   install -d -o monita -g monita -m 0700 /var/lib/monita /var/backups/monita
   ```

2. Write the configuration. Every variable is described in [docs/technical-specs/11-environment-configuration.md](docs/technical-specs/11-environment-configuration.md).

   ```sh
   install -d -m 0755 /etc/monita
   install -m 0600 -o monita deploy/monita.env.example /etc/monita/monita.env
   ${EDITOR:-vi} /etc/monita/monita.env      # set MONITA_BASE_URL, MONITA_SMTP_URL, MONITA_PING_URL
   ```

3. Create the first super admin, then start the service:

   ```sh
   set -a; . /etc/monita/monita.env; set +a
   sudo -u monita -E monita admin create --email you@company.co.id
   install -m 0644 deploy/monita.service /etc/systemd/system/monita.service
   systemctl daemon-reload && systemctl enable --now monita
   curl -s http://127.0.0.1:8080/healthz
   ```

4. Put the company reverse proxy in front for HTTPS, and set `MONITA_TRUST_PROXY=true` so rate limits see the real client IP. Keep `MONITA_BIND=127.0.0.1:8080`: if other hosts can reach the port, they can forge `X-Forwarded-For`.

### Backups and restore

Monita writes `monita-YYYYMMDD.db` to `MONITA_BACKUP_DIR` at 01:00 UTC and keeps the newest `MONITA_BACKUP_KEEP` (7). To restore one:

```sh
systemctl stop monita
mv /var/lib/monita/monita.db /var/lib/monita/monita.db.broken
rm -f /var/lib/monita/monita.db-wal /var/lib/monita/monita.db-shm
install -m 0600 -o monita /var/backups/monita/monita-20261016.db /var/lib/monita/monita.db
systemctl start monita
```

A backup is a plain SQLite file: `sqlite3 monita-20261016.db 'PRAGMA integrity_check'` checks it.

### Watch Monita from outside

Monita cannot alert about its own outage. Run a second Monita at another site and give each a heartbeat on the other:

1. On Monita B, create a heartbeat named `monita-a`, period 1 minute, grace 5 minutes, severity critical.
2. On Monita A, set `MONITA_PING_URL` to that heartbeat's ping URL and restart. A pings it at start and every 60 seconds.
3. Do the same in the other direction.

If one site loses power, the other sends "monita-a missed its heartbeat". A free Better Stack heartbeat works the same way in place of Monita B. `GET /healthz` returns 503 when the database fails or the scheduler stops ticking, for a load balancer or an uptime check.

### Lost super admin password

```sh
systemctl stop monita        # the running service locks the database file
set -a; . /etc/monita/monita.env; set +a
sudo -u monita -E monita admin reset-password --email you@company.co.id
systemctl start monita
```

The command reads the new password from stdin and signs the admin out everywhere. A TL gets a new invite from a super admin instead.

### Upgrade

Install the new binary over the old one and run `systemctl restart monita`. Migrations run at start. Take a backup first: `systemctl stop monita && cp /var/lib/monita/monita.db /var/backups/monita/pre-upgrade.db`.

## Releases

Push a `v*` tag that matches the version in `Cargo.toml`. [release.yml](.github/workflows/release.yml) builds the static Linux binary and opens a draft GitHub release.
