# Roadmap

Monita 1.1 is stable and in maintenance. This roadmap states the direction. It is a statement of intent, not a contract, and it carries no dates: a small team sets its pace by capacity.

For released work, see [CHANGELOG.md](CHANGELOG.md).

## Now: maintenance

- Keep dependencies current and the supply chain checked: monthly Dependabot updates, actions pinned by commit SHA, `cargo deny` in CI, and an attestation on every release.
- Fix reported bugs, and answer security reports within the times in [SECURITY.md](SECURITY.md).
- Keep the footprint small: one binary, one SQLite file, and the memory and size targets in [spec 08](docs/technical-specs/08-non-functional-requirements.md).

## Done in 1.1

[The v1.1 decision](docs/decisions/2026-10-19-v1.1-scope.md) closed the gaps a comparison with Uptime Kuma and Gatus found:

- TCP and DNS checks ([#9](https://github.com/PT-Perkasa-Pilar-Utama/monita/issues/9)) and ICMP ping ([#7](https://github.com/PT-Perkasa-Pilar-Utama/monita/issues/7)).
- Discord, Telegram, ntfy and PagerDuty alerts ([#10](https://github.com/PT-Perkasa-Pilar-Utama/monita/issues/10)).
- Maintenance windows ([#11](https://github.com/PT-Perkasa-Pilar-Utama/monita/issues/11)).
- Two-factor sign-in ([#12](https://github.com/PT-Perkasa-Pilar-Utama/monita/issues/12)).
- A warning before a TLS certificate expires ([#8](https://github.com/PT-Perkasa-Pilar-Utama/monita/issues/8)).
- Checks from more than one place, with `monita probe` ([#6](https://github.com/PT-Perkasa-Pilar-Utama/monita/issues/6)).
- Alert cards instead of fixed alert settings, ticket cards, and a global Telegram bot that a super admin sets up once.
- Pause for a monitor or heartbeat, and batch pause, resume and delete.
- API key scopes (`ref_ids`, `read`, `write`) and a project key API under `/api/v1/projects/{slug}`.
- Pins for projects, monitors and heartbeats.
- Reclassify an incident as planned work or a false alarm.
- Edit a maintenance window.
- SQLite through rusqlite in place of Turso.
- The security audit fixes of 2026-10-27, in the [1.1.0 changelog](CHANGELOG.md).

Smaller known gaps are [GitHub issues](https://github.com/PT-Perkasa-Pilar-Utama/monita/issues).

## Later, with a decision first

Each of these needs a decision in [docs/decisions/](docs/decisions/) before a pull request:

- DNS record types other than A and AAAA, which need a resolver library.
- Microsoft Teams Adaptive Cards as a webhook format.
- Recurring maintenance windows, and two-factor sign-in required for everyone.
- An alert when a location's probe goes silent, and charts per location.
- Text-message or phone alerts and on-call rotations; single sign-on; files attached to tickets.

## Proposing changes

Open an issue to discuss direction, or a pull request for a concrete change. See [CONTRIBUTING.md](CONTRIBUTING.md) and [GOVERNANCE.md](GOVERNANCE.md).
