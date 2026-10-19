# Roadmap

Monita 1.0 is stable and in maintenance. This roadmap states the direction. It is a statement of intent, not a contract, and it carries no dates: a small team sets its pace by capacity.

For released work, see [CHANGELOG.md](CHANGELOG.md).

## Now: maintenance

- Keep dependencies current and the supply chain checked: monthly Dependabot updates, actions pinned by commit SHA, `cargo deny` in CI, and an attestation on every release.
- Fix reported bugs, and answer security reports within the times in [SECURITY.md](SECURITY.md).
- Keep the footprint small: one binary, one SQLite file, and the memory and size targets in [spec 08](docs/technical-specs/08-non-functional-requirements.md).

## Next: the open list

[docs/followups.md](docs/followups.md) holds the known gaps: browser tests for rare states, form errors that should clear as you type, and a few small API answers to settle. Work starts there.

## Out of scope for 1.x

Version 1 leaves these out on purpose. The landing page and [the scope decision](docs/decisions/2026-10-16-monita-v1-scope.md) explain why. Any of them would start with a new decision in [docs/decisions/](docs/decisions/), not a pull request.

- Checks from more than one place.
- TCP, ping and DNS checks.
- Text-message or phone alerts, and on-call rotations.
- Single sign-on and two-factor sign-in.
- Files attached to tickets.

## Proposing changes

Open an issue to discuss direction, or a pull request for a concrete change. See [CONTRIBUTING.md](CONTRIBUTING.md) and [GOVERNANCE.md](GOVERNANCE.md).
