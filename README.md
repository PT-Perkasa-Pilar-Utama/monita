<div align="center">

<img src="docs/assets/logo.svg" alt="Monita" width="236">

**Monita checks your team's websites and scheduled jobs, alerts your team when one fails, and shows the outage on a public status page.**
You run it on your own server as one program, and it keeps all its data in one SQLite file.

[![CI](https://github.com/PT-Perkasa-Pilar-Utama/monita/actions/workflows/ci.yml/badge.svg)](https://github.com/PT-Perkasa-Pilar-Utama/monita/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-c4258f)](LICENSE)

**[Website](https://pt-perkasa-pilar-utama.github.io/monita/)** &nbsp;&nbsp;
**[Try the demo](https://pt-perkasa-pilar-utama.github.io/monita/demo/)** &nbsp;&nbsp;
**[Docs](https://pt-perkasa-pilar-utama.github.io/monita/docs/)** &nbsp;&nbsp;
**[Install](#install)**

</div>

Monita does the same job as a hosted uptime service such as Better Stack, but it runs on a server
that your team controls.

- A monitor requests one of your URLs every 30 seconds to 5 minutes.
- A heartbeat expects your cron job to call a Monita URL on a schedule.
- When a monitor fails twice in a row, or a heartbeat call does not arrive in time, Monita opens an
  incident. It emails the addresses you list for the project, and it posts to your chat if you
  add a webhook.
- Each project has a public status page. The page shows 90 days of uptime and the updates your team
  writes during an incident.
- Users can report a bug from the status page, and the tech lead sorts the reports on the ticket board.

The [demo](https://pt-perkasa-pilar-utama.github.io/monita/demo/) is a saved copy of the real pages
with sample data. You can click through it, but nothing you do changes anything.

![A monitor page: uptime streak, last check, mean response time, a response-time chart and availability](docs/assets/screens/monitor.png)

| Status page | Tickets |
| --- | --- |
| ![A project's public status page: 90 days of uptime per service and the incident timeline](docs/assets/screens/status.png) | ![The ticket board: triage, backlog, in progress, done](docs/assets/screens/tickets.png) |

## Terms

This README and the guide use each of these words with one meaning.

| Term | Meaning |
| --- | --- |
| Project | One system your team runs. It has its own monitors, heartbeats, status page and tickets. |
| Monitor | A URL that Monita requests on a fixed interval. |
| Check | One request that a monitor makes. It passes or fails. |
| Heartbeat | A URL that a cron job calls when it runs. Monita expects the call before a deadline. |
| Ping | One call from a cron job to its heartbeat URL. |
| Incident | A record that something is down. Monita opens an automatic incident itself. A tech lead opens a manual incident by hand. |
| Alert | An email or a webhook that Monita sends when an incident opens or closes. |
| Webhook | A message that Monita posts to a URL, for example a Slack or Teams channel. |
| Status page | The public page of a project. Anyone with the link can read it. |
| Ticket | A bug report from a user. It moves through triage, backlog, in progress and done. |
| Ticket board | The tech lead's view of every ticket in a project. The public board on the status page leaves out tickets in triage. |
| Tech lead | A user who owns projects. A tech lead sees only their own projects. |
| Super admin | A user who invites tech leads and can see every project. |

## Install

Download the `monita` binary from [Releases](https://github.com/PT-Perkasa-Pilar-Utama/monita/releases)
to a Linux server, then run three commands:

```sh
install -m 0755 monita /usr/local/bin/monita
sudo monita setup
sudo monita service install
```

`monita setup` asks for the public URL, the mail server and the first super admin.
`monita service install` makes Monita start now and after every reboot.

Monita listens on `127.0.0.1:8080`. Put a web server that handles HTTPS, such as Caddy or nginx, in
front of it. The [install guide](https://pt-perkasa-pilar-utama.github.io/monita/docs/install/)
explains these steps. The [operations guide](https://pt-perkasa-pilar-utama.github.io/monita/docs/operations/)
explains backups, restores and upgrades.

## Send a ping from a cron job

```sh
URL=https://monita.example.com/ping/<token>
if ! out=$(backup.sh 2>&1); then
  printf '%s' "$out" | curl -fsS -m 10 --data-binary @- "$URL/fail"
  exit 1
fi
curl -fsS -m 10 "$URL"
```

If the job does not call the URL before its deadline, Monita opens an incident. A call to `/fail`
opens an incident at once. Monita stores the first 1 KB of that request as the cause of the
incident, so the command above sends the job's error output.

## Docs

The [documentation](https://pt-perkasa-pilar-utama.github.io/monita/docs/) explains how to start,
how to connect your jobs and tools, and how to run Monita in production. Its source files are in
[docs/guide/](docs/guide/).

| Start | Connect | Reference |
| --- | --- | --- |
| [Getting started](docs/guide/getting-started.md) | [Heartbeats and cron](docs/guide/heartbeats.md) | [Technical specs](docs/technical-specs/_index.md) |
| [Install](docs/guide/install.md) | [Status JSON](docs/guide/status-page.md) | [API specs](docs/api-specs/_index.md) |
| [Configuration](docs/guide/configuration.md) | [Tickets API](docs/guide/tickets.md) | [Design system](docs/design-system/) |
| [Operations](docs/guide/operations.md) | [Webhooks](docs/guide/incidents.md) | [Decisions](docs/decisions/) |

## Contributing

You need Rust, Bun 1.4 and Google Chrome. The Rust version in `rust-toolchain.toml` installs itself.

```sh
make setup    # install the git hooks and the browser-test packages
make run      # start Monita on http://127.0.0.1:8080
make check    # check formatting and lints, then run the Rust tests
make e2e      # run the browser tests on port 18080
```

[CLAUDE.md](CLAUDE.md) lists the project rules and commands.
[seeds/README.md](seeds/README.md) lists the sample accounts.

## License

Monita is released under the MIT license. Copyright PT. Perkasa Pilar Utama.
