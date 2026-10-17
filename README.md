<div align="center">

<img src="docs/assets/logo.svg" alt="Monita" width="236">

**Uptime monitoring, cron heartbeats, incidents, status pages and bug tickets for every project in your team.**
Monita runs as a single self-hosted binary and keeps all its data in a SQLite file.

[![CI](https://github.com/PT-Perkasa-Pilar-Utama/monita/actions/workflows/ci.yml/badge.svg)](https://github.com/PT-Perkasa-Pilar-Utama/monita/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-c4258f)](LICENSE)

**[Website](https://pt-perkasa-pilar-utama.github.io/monita/)** &nbsp;&nbsp;
**[Try the demo](https://pt-perkasa-pilar-utama.github.io/monita/demo/)** &nbsp;&nbsp;
**[Docs](https://pt-perkasa-pilar-utama.github.io/monita/docs/)** &nbsp;&nbsp;
**[Install](#install)**

</div>

Monita checks your services and cron jobs and tells the right people when something breaks. It
covers what a team usually buys from Better Stack, and you host it yourself.

- HTTP monitors request your URLs every 30 seconds to 5 minutes, and heartbeats wait for your cron
  jobs to report in.
- Two failed checks in a row, or a missed heartbeat, open an incident and send email and webhook
  alerts.
- Each project gets a public status page that shows 90 days of uptime and the updates you post.
- Users can report bugs through the status page, and the project's tech lead sorts them on a
  kanban.

The [demo](https://pt-perkasa-pilar-utama.github.io/monita/demo/) is a read-only snapshot of the
real app on sample data.

![A monitor page: uptime streak, last check, mean response time, a response-time chart and availability](docs/assets/screens/monitor.png)

| Status page | Tickets |
| --- | --- |
| ![A project's public status page: 90 days of uptime per service and the incident timeline](docs/assets/screens/status.png) | ![The ticket kanban: triage, backlog, in progress, done](docs/assets/screens/tickets.png) |

## Install

On a Linux VM, with the binary from [Releases](https://github.com/PT-Perkasa-Pilar-Utama/monita/releases):

```sh
install -m 0755 monita /usr/local/bin/monita
sudo monita setup             # public URL, SMTP, folders, first super admin
sudo monita service install   # systemd unit, started now and at every boot
```

Put your HTTPS reverse proxy in front of `127.0.0.1:8080`. Backups, restore and upgrades:
[Install](https://pt-perkasa-pilar-utama.github.io/monita/docs/install/) and [Operations](https://pt-perkasa-pilar-utama.github.io/monita/docs/operations/).

## Ping from a cron job

```sh
if backup.sh; then curl -fsS -m 10 https://monita.example.com/ping/<token>
else curl -fsS -m 10 https://monita.example.com/ping/<token>/fail; fi
```

No ping within the period you set opens an incident, and so does `/fail` at once. A failing job can
send its error as the request body: the first 1 KB becomes the incident's cause.

## Docs

The [documentation](https://pt-perkasa-pilar-utama.github.io/monita/docs/) covers getting started, every integration, and running Monita in production. Its sources are in [docs/guide/](docs/guide/).

| Start | Integrate | Reference |
| --- | --- | --- |
| [Getting started](docs/guide/getting-started.md) | [Heartbeats and cron](docs/guide/heartbeats.md) | [Technical specs](docs/technical-specs/_index.md) |
| [Install](docs/guide/install.md) | [Status JSON](docs/guide/status-page.md) | [API specs](docs/api-specs/_index.md) |
| [Configuration](docs/guide/configuration.md) | [Tickets API](docs/guide/tickets.md) | [Design system](docs/design-system/) |
| [Operations](docs/guide/operations.md) | [Webhooks](docs/guide/incidents.md) | [Decisions](docs/decisions/) |

## Contributing

Rust (the toolchain in `rust-toolchain.toml` installs itself), Bun 1.4 and Google Chrome:

```sh
make setup    # git hooks and e2e packages
make run      # http://127.0.0.1:8080
make check    # format, clippy, unit and integration tests
make e2e      # browser tests on port 18080
```

[CLAUDE.md](CLAUDE.md) lists every rule and command. Seeded accounts: [seeds/README.md](seeds/README.md).

## License

MIT. Copyright PT. Perkasa Pilar Utama.
