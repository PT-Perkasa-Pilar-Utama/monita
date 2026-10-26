# Changelog

All notable changes to Monita are listed in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and Monita follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.1.0] - 2026-10-21

More ways to check, alert and plan: TCP, DNS and ping monitors, certificate warnings, checks from more than one place, Discord, Telegram, ntfy and PagerDuty alerts, maintenance windows, and two-factor sign-in. Migrations 0024 to 0040 run on start; 0029 rebuilds the `monitors` table, and 0039 rebuilds `alert_cards`.

### Added

- Locations: checks from more than one place. A super admin adds a location and runs `monita probe` on another machine with its token. A monitor checks from this server and any locations, and counts a check as failed only when a chosen number of places agree ([#6](https://github.com/PT-Perkasa-Pilar-Utama/monita/issues/6)). Migrations 0030 and 0031. Design: `docs/technical-specs/14-locations.md`.
- Ping monitors: one ICMP echo per check through an unprivileged socket, with no root and no capability. Where the system refuses that socket, and on Windows, Monita runs the system `ping` instead. Docker opens the socket by default ([#7](https://github.com/PT-Perkasa-Pilar-Utama/monita/issues/7)). Migration 0029 rebuilds `monitors`.
- A warning before an HTTPS monitor's certificate expires: 7, 14 or 30 days ahead, once per certificate, through the warning route, without an incident ([#8](https://github.com/PT-Perkasa-Pilar-Utama/monita/issues/8)). Migration 0028.
- Two-factor sign-in with an authenticator app, recovery codes, and a super admin action to turn it off for a user ([#12](https://github.com/PT-Perkasa-Pilar-Utama/monita/issues/12)). An account takes 10 wrong codes per 15 minutes. Migration 0027.
- Maintenance windows: a tech lead plans one over chosen monitors and heartbeats. During it, failures open no incident and send no alert, and the status page shows Maintenance with the team's message ([#11](https://github.com/PT-Perkasa-Pilar-Utama/monita/issues/11)). Migration 0026.
- Discord, Telegram, ntfy and PagerDuty as webhook formats for each severity ([#10](https://github.com/PT-Perkasa-Pilar-Utama/monita/issues/10)). Migration 0025.
- Pause a monitor or heartbeat instead of deleting it. While paused, nothing checks it, nothing opens and no alert goes out. A heartbeat keeps its pings. An open incident closes on pause, and the system leaves the status page until it resumes. Migration 0040.
- **Plan maintenance** on a monitor's or heartbeat's page opens the maintenance form with it already picked.
- The project list opens on a chart of the last 24 hours, a line for each project on the page: its monitors' mean response time, red where a monitor was down, and a lane of its heartbeats' pings. The card and list views remain, and the view you pick is remembered.
- Filter the project list by **State**: Down, Issue or Up, as the badges read.
- The Overview charts the last 24 hours of every monitor and heartbeat on one time axis. The 5 monitors that need you first draw as response-time lines on a log scale, the rest faint behind them. Heartbeats draw as lanes of ping dots, failures in red. Hover for every value in a 15-minute slot; select a line or lane to open it. It reads again every 30 seconds and has a Refresh button.
- `monita admin reset-sign-in --email` is the way back in for anyone who lost both the authenticator and the recovery codes. It turns two-factor off, stops the old password, and emails a link to set a new one. It prints the link too.
- **How API keys work** on the API keys page: what a key is for, how to send it, and each endpoint it opens.
- Pictures show in the user list and beside a project's owner.
- TCP port and DNS name monitors beside HTTP ([#9](https://github.com/PT-Perkasa-Pilar-Utama/monita/issues/9)). Migration 0024.
- **User settings**, **Password**: change your own password with the current one. Your other sessions sign out.
- **User settings**, **Profile**: a picture the top bar shows in place of your initial. Migration 0034.
- A global Telegram bot for every project: a super admin sets it up once in **Admin**, **Telegram**, and a project picks **Global Telegram** and connects a chat with a link or QR code. The card form shows the link at once and says when the chat is connected. `MONITA_TELEGRAM_API` points at a self-hosted Bot API server. Migration 0036.
- Discord and Telegram alerts are laid out for reading: a coloured title that links to Monita, the project, and the cause or how long it was down. Discord posts them as Monita.

### Changed

- **Ticket alerts** are alert cards too, in their own section. Every project has three fixed owner cards: the daily digest, each new ticket, and each ticket move. A new project starts with all three off. A project adds up to 10 ticket cards of its own: an email list or a webhook in JSON, Discord, Telegram, the global Telegram bot or ntfy, each listening to new tickets, moves or both. Migrations 0038 and 0039 convert the old settings: the digest keeps its state, and a saved ticket webhook becomes a card.
- Passwords need 8 characters or more, down from 12.
- **Alert cards** replace the Critical and Warning sections. Every project keeps two fixed cards that email its owner, one per level, and adds up to 10 of its own: an email list or a webhook in any format, each listening to critical, warning or both, each with its own switch and test. A card needs no name: it takes its kind, or its Telegram chat. Migration 0037 converts the old settings; nobody who got alerts stops getting them.
- A new project's critical alerts no longer start with the webhook ticked. Neither severity calls a webhook until its owner ticks it and sets a URL. Migration 0033 unticks existing routes that have no URL.
- The project list shows the newest projects first. Sort by **Name** to get the old order.
- A new project's status page shows the Changelog tab. Existing projects keep their setting.
- Two-factor setup shows a QR code to scan. The key and the app link stay below it. A key not confirmed within 10 minutes expires. Migration 0035.
- **Documentation** moved from the top bar into the avatar menu, so it no longer takes clicks meant for the links beside it.
- The avatar menu's **Two-factor sign-in** became **User settings**, an area with its own side menu. While two-factor is off, a note under the top bar says so until you close it in that browser.

### Fixed

- Pages that listed a project's monitors read every project's checks. On a large install the Monitors list took over a second; it now reads only its own monitors (6 ms in a test with 290 000 checks).
- The project list showed green bars for hours before a monitor or heartbeat existed. Those hours now show no data.
- Mail to Monita's own sender address is dropped, so alerts no longer loop back into the sending mailbox. A user who signs in with that address still gets mail.
- A status page with no public system, or none checked yet, no longer says "All systems operational". It says so in grey, and `overall` in its JSON is `unknown`.
- The Telegram admin page shows clearly whether a bot is set.
- The custom status page's notes are a labelled table instead of a list.
- Recovery codes keep their spacing, and a wrong two-factor code's error uses the whole row.

- Signing in at `http://127.0.0.1:7466` no longer fails with "wrong origin" when Monita's address is the default `http://localhost:7466`. The start-up log names the address to open.
- The install steps put `admin create` last in its block, so a pasted block no longer types the next command into the password prompt.
- Sending a new invite stops the old password at once, as the guide says. Before, it kept working until the new link was used.
- The online demo's side panels (a ticket, a new or edited monitor) close with their X, Cancel, the backdrop or Escape.

## [1.0.1] - 2026-10-19

### Added

- A container image, `ghcr.io/pt-perkasa-pilar-utama/monita`, for `linux/amd64` and `linux/arm64`: the release's static binary on a distroless base, as a non-root user, with data in `/data`.
- `SECURITY.md`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `GOVERNANCE.md`, `ROADMAP.md`, this changelog, issue forms, a pull request template and code owners.
- `docs/GLOSSARY.md`, which replaces the README's Terms table.
- Release archives for Linux on arm64 and for macOS on Apple silicon and Intel, beside Linux on amd64. The v1.0.0 release has them too, built from its tag.
- Windows release archives for amd64 and arm64. `monita service` stays Linux only; on Windows, run `monita serve --env-file`.
- The release workflow builds and boots every target from a branch with `build_only`, without releasing anything.

### Changed

- Dependabot opens grouped minor and patch updates once a month, and now covers the web packages too.
- CI skips the browser tests and the service smoke when a push changes only docs and Markdown, and the Rust check skips itself when its inputs passed before.

## [1.0.0] - 2026-10-19

The first stable release. See the [release notes](https://github.com/PT-Perkasa-Pilar-Utama/monita/releases/tag/v1.0.0) for the upgrade steps from 0.1.0.

### Added

- Every page in one React and shadcn interface built into the binary.
- A Supervisor role that sees what a super admin sees and changes nothing.
- A changelog per project, shown on its status page.
- Status page tabs for uptime, incidents, known issues, tickets and the changelog, and a downloadable HTML template for a status page of your own.
- Checks with a method, headers, a body and a sealed secret; a failure count from 1 to 99; alerts and automatic incidents switched per check.
- Resolving an automatic incident by hand.
- Ticket alerts with their own digest and webhook, and team-filed tickets on a board with drag and drop.
- The mail server, time zone and rate limits set in the app.
- Moni, the mascot, and `make brand` for the logo, Moni and the scenes as files.

### Changed

- The release archive is named `monita_linux_amd64.tar.gz`, without the version.
- Monita refuses to start when `MONITA_SMTP_URL`, `MONITA_MAIL_FROM`, `MONITA_LOG` or `MONITA_PING_URL` does not parse, and names the variable.
- `MONITA_MAIL_FROM` defaults to `Monita <monita@{host of MONITA_BASE_URL}>`.
- A monitor URL or a project homepage may not hold a user name or password.

### Fixed

- A new project's alerts email its owner. Migration 0023 fills the routes that sent nothing.
- Sign-in timing no longer shows which emails have accounts.
- Two super admins acting on each other at once can no longer leave none.
- Invite links and wrong API keys are rate-limited per address; sign-in has its own counter.
- A reporter's ticket title cannot add a link or a mention to the team's chat.
- The database and its WAL are mode 0600 whenever Monita opens them.

## [0.1.0] - 2026-10-17

The first release: HTTP monitors, heartbeats, automatic and manual incidents, email and webhook alerts through an outbox, public status pages with a status JSON, bug tickets with ref IDs, and the `monita` command line for setup and the systemd service.

[Unreleased]: https://github.com/PT-Perkasa-Pilar-Utama/monita/compare/v1.1.0...HEAD
[1.1.0]: https://github.com/PT-Perkasa-Pilar-Utama/monita/compare/v1.0.1...v1.1.0
[1.0.1]: https://github.com/PT-Perkasa-Pilar-Utama/monita/compare/v1.0.0...v1.0.1
[1.0.0]: https://github.com/PT-Perkasa-Pilar-Utama/monita/compare/v0.1.0...v1.0.0
[0.1.0]: https://github.com/PT-Perkasa-Pilar-Utama/monita/releases/tag/v0.1.0
