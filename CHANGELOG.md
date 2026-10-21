# Changelog

All notable changes to Monita are listed in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and Monita follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- A warning before an HTTPS monitor's certificate expires: 7, 14 or 30 days ahead, once per certificate, through the warning route, without an incident ([#8](https://github.com/PT-Perkasa-Pilar-Utama/monita/issues/8)). Migration 0028.
- Two-factor sign-in with an authenticator app, recovery codes, and a super admin action to turn it off for a user ([#12](https://github.com/PT-Perkasa-Pilar-Utama/monita/issues/12)). Migration 0027.
- Maintenance windows: a tech lead plans one over chosen monitors and heartbeats. During it, failures open no incident and send no alert, and the status page shows Maintenance with the team's message ([#11](https://github.com/PT-Perkasa-Pilar-Utama/monita/issues/11)). Migration 0026.
- Discord, Telegram, ntfy and PagerDuty as webhook formats for each severity ([#10](https://github.com/PT-Perkasa-Pilar-Utama/monita/issues/10)). Migration 0025.
- TCP port and DNS name monitors beside HTTP ([#9](https://github.com/PT-Perkasa-Pilar-Utama/monita/issues/9)). Migration 0024.

### Fixed

- Signing in at `http://127.0.0.1:7466` no longer fails with "wrong origin" when Monita's address is the default `http://localhost:7466`. The start-up log names the address to open.
- The install steps put `admin create` last in its block, so a pasted block no longer types the next command into the password prompt.

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

[Unreleased]: https://github.com/PT-Perkasa-Pilar-Utama/monita/compare/v1.0.1...HEAD
[1.0.1]: https://github.com/PT-Perkasa-Pilar-Utama/monita/compare/v1.0.0...v1.0.1
[1.0.0]: https://github.com/PT-Perkasa-Pilar-Utama/monita/compare/v0.1.0...v1.0.0
[0.1.0]: https://github.com/PT-Perkasa-Pilar-Utama/monita/releases/tag/v0.1.0
