<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/assets/brand/logo-on-dark.svg">
  <img src="docs/assets/brand/logo.svg" alt="Monita" width="228">
</picture>

### Site reliability for every project your team ships.

Monita checks your team's websites and scheduled jobs, alerts the people you choose when one fails, and shows the outage on a public status page. It runs on your own server as one program with one SQLite file.

[![CI](https://github.com/PT-Perkasa-Pilar-Utama/monita/actions/workflows/ci.yml/badge.svg)](https://github.com/PT-Perkasa-Pilar-Utama/monita/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-c4258f)](LICENSE)

**[Website](https://pt-perkasa-pilar-utama.github.io/monita/)** &nbsp;&nbsp;
**[Try the demo](https://pt-perkasa-pilar-utama.github.io/monita/demo/)** &nbsp;&nbsp;
**[Docs](https://pt-perkasa-pilar-utama.github.io/monita/docs/)** &nbsp;&nbsp;
**[Install](#install)**

<img src="docs/assets/brand/moni-friends.svg" alt="Moni, the Monita mascot, and friends at play: flying a kite, playing catch, stacking uptime blocks and blowing bubbles" width="100%">

</div>

- **Monitors** check a URL, a TCP port, a DNS name or a host by ping every 30 s to 5 min, and warn before an HTTPS certificate expires.
- **Heartbeats** wait for your cron jobs to call in.
- **Incidents** open after two failed checks or a missed heartbeat, and alert by email, Slack, Google Chat, Discord, Telegram, ntfy or PagerDuty.
- **Status pages** show 90 days of uptime and the updates your team posts.
- **Tickets** collect bug reports from the status page or from your app.
- **Maintenance windows, pause and locations** keep planned work quiet and check from more than one place.
- **Roles, two-factor sign-in and scoped API keys** let one Monita serve the whole IT department.

Click through the [demo](https://pt-perkasa-pilar-utama.github.io/monita/demo/) first. It runs on sample data and saves nothing.

![A monitor page: uptime streak, last check, mean response time, a response-time chart and availability](docs/assets/screens/monitor.png)

| Status page | Tickets |
| --- | --- |
| ![A project's public status page: 90 days of uptime per service and the incident timeline](docs/assets/screens/status.png) | ![The ticket board: triage, backlog, in progress, done](docs/assets/screens/tickets.png) |

## Install

On a 64-bit Linux server with systemd, 512 MB of memory and 5 GB of disk. On Arm, use `monita_linux_arm64.tar.gz`.

```sh
curl -fsSLO https://github.com/PT-Perkasa-Pilar-Utama/monita/releases/latest/download/monita_linux_amd64.tar.gz
tar -xzf monita_linux_amd64.tar.gz
sudo install -m 0755 monita /usr/local/bin/monita
sudo monita setup
sudo monita service install
```

`monita setup` asks for the public URL, the mail server and the first super admin. Put Caddy or nginx in front of `127.0.0.1:7466` for HTTPS.

To try it on a laptop with Docker, create the first account (it asks for a password of at least 8 characters):

```sh
docker volume create monita-data
docker run --rm -it -v monita-data:/data -e MONITA_ENV=dev ghcr.io/pt-perkasa-pilar-utama/monita:1 admin create --email you@company.co.id
```

Then start Monita and open `http://localhost:7466`:

```sh
docker run -d --name monita -p 127.0.0.1:7466:7466 -v monita-data:/data -e MONITA_ENV=dev ghcr.io/pt-perkasa-pilar-utama/monita:1
```

The [install guide](https://pt-perkasa-pilar-utama.github.io/monita/docs/install/) covers a Docker server, macOS, Windows, upgrades and building from source.

## Send a ping from a cron job

```sh
URL=https://monita.example.com/ping/<token>
if ! out=$(backup.sh 2>&1); then
  printf '%s' "$out" | curl -fsS -m 10 --data-binary @- "$URL/fail"
  exit 1
fi
curl -fsS -m 10 "$URL"
```

A missed deadline opens an incident. A call to `/fail` opens one at once, with the first 1 KB of the body as its cause.

## Benchmark

Each tool checked 500 URLs every 60 s against one local nginx, alone in Docker on an Apple M1 (2 vCPU), measured with `docker stats` on 2026-10-20.

| 500 checks a minute | Monita 1.0.1 | Uptime Kuma 2.5.6 | Gatus (latest on 2026-10-20) |
| --- | --- | --- | --- |
| Memory under load (median) | 21 MiB | 160 MiB | 44 MiB |
| Memory idle | 18 MiB | 124 MiB | 5 MiB |
| CPU, share of one core | 0.8% | 6.6% | 1.8% |
| Checks done per minute, of 500 | 498 | 393 | 493 |
| Image size (arm64, as Docker reports it) | 10.9 MB | 603 MB | 24.5 MB |

## Docs

The [documentation](https://pt-perkasa-pilar-utama.github.io/monita/docs/) is built from [docs/guide/](docs/guide/).

| Start | Connect | Reference |
| --- | --- | --- |
| [Getting started](docs/guide/getting-started.md) | [Heartbeats and cron](docs/guide/heartbeats.md) | [Technical specs](docs/technical-specs/_index.md) |
| [Install](docs/guide/install.md) | [Alerts and webhooks](docs/guide/alerts.md) | [API specs](docs/api-specs/_index.md) |
| [Configuration](docs/guide/configuration.md) | [Status JSON](docs/guide/status-page.md) | [Limits](docs/guide/limits.md) |
| [Operations](docs/guide/operations.md) | [Tickets API](docs/guide/tickets.md) | [Design system](docs/design-system/) |
| [Locations](docs/guide/locations.md) | [Project API](docs/guide/api.md) | [Decisions](docs/decisions/) |

## Contributing

You need Rust, Bun 1.4, a C compiler and Google Chrome. The Rust version in `rust-toolchain.toml` installs itself.

```sh
make setup    # install the git hooks and the browser-test packages
make run      # start Monita on http://127.0.0.1:7466
make check    # check formatting and lints, then run the Rust tests
make e2e      # run the browser tests on port 18080
```

Read [CONTRIBUTING.md](CONTRIBUTING.md) before you send a change. [CLAUDE.md](CLAUDE.md) holds the project rules and [docs/GLOSSARY.md](docs/GLOSSARY.md) the terms.

Report a security problem privately, as [SECURITY.md](SECURITY.md) describes. Everyone who takes part follows the [Code of Conduct](CODE_OF_CONDUCT.md). [GOVERNANCE.md](GOVERNANCE.md) names the maintainers, [ROADMAP.md](ROADMAP.md) the direction and [CHANGELOG.md](CHANGELOG.md) each release.

## License

Monita is released under the MIT license. Copyright PT. Perkasa Pilar Utama.

The binary embeds the Inter and Geist Mono fonts, which are under the SIL Open Font License 1.1. Their licences are in [licenses/](licenses/) and in every release archive.
