# Roadmap

Monita 1.0 is stable and in maintenance. This roadmap states the direction. It is a statement of intent, not a contract, and it carries no dates: a small team sets its pace by capacity.

For released work, see [CHANGELOG.md](CHANGELOG.md).

## Now: maintenance

- Keep dependencies current and the supply chain checked: monthly Dependabot updates, actions pinned by commit SHA, `cargo deny` in CI, and an attestation on every release.
- Fix reported bugs, and answer security reports within the times in [SECURITY.md](SECURITY.md).
- Keep the footprint small: one binary, one SQLite file, and the memory and size targets in [spec 08](docs/technical-specs/08-non-functional-requirements.md).

## Next: 1.1

[The v1.1 decision](docs/decisions/2026-10-19-v1.1-scope.md) takes four of the gaps a comparison with Uptime Kuma and Gatus found:

- TCP and DNS checks ([#9](https://github.com/PT-Perkasa-Pilar-Utama/monita/issues/9)).
- Discord, Telegram, ntfy and PagerDuty alerts ([#10](https://github.com/PT-Perkasa-Pilar-Utama/monita/issues/10)).
- Maintenance windows ([#11](https://github.com/PT-Perkasa-Pilar-Utama/monita/issues/11)).
- Two-factor sign-in ([#12](https://github.com/PT-Perkasa-Pilar-Utama/monita/issues/12)).

Smaller known gaps are [GitHub issues](https://github.com/PT-Perkasa-Pilar-Utama/monita/issues).

## Later, with a decision first

Each of these needs a decision in [docs/decisions/](docs/decisions/) before a pull request:

- Checks from more than one place: a v2 design with probe agents ([#6](https://github.com/PT-Perkasa-Pilar-Utama/monita/issues/6)).
- ICMP ping, which needs a raw-socket privilege ([#7](https://github.com/PT-Perkasa-Pilar-Utama/monita/issues/7)).
- TLS certificate expiry, which needs a separate TLS handshake ([#8](https://github.com/PT-Perkasa-Pilar-Utama/monita/issues/8)).
- Text-message or phone alerts and on-call rotations; single sign-on; files attached to tickets.

## Proposing changes

Open an issue to discuss direction, or a pull request for a concrete change. See [CONTRIBUTING.md](CONTRIBUTING.md) and [GOVERNANCE.md](GOVERNANCE.md).
