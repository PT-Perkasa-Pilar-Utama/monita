# Contributing to Monita

This guide covers how to set up Monita, which checks a change must pass, and how a release is cut.

## Contents

- [Before you start](#before-you-start)
- [Set up](#set-up)
- [Make a change](#make-a-change)
- [Checks](#checks)
- [Open a pull request](#open-a-pull-request)
- [Cut a release](#cut-a-release)
- [Report a problem](#report-a-problem)

## Before you start

- For a bug, a feature or a question, open an issue first, so nobody builds the same thing twice.
- For a security problem, do not open an issue. Follow [SECURITY.md](SECURITY.md).
- Everyone who takes part follows the [Code of Conduct](CODE_OF_CONDUCT.md).

## Set up

You need:

| Tool | Version | Note |
| --- | --- | --- |
| Rust | 1.97.1 | Install [rustup](https://rustup.rs). `rust-toolchain.toml` picks the version. |
| Bun | 1.4.2 | CI pins this version. It builds the web pages and runs the browser tests. |
| C compiler | Any | `build-essential` on Debian or Ubuntu, Xcode Command Line Tools on macOS. |
| Google Chrome | Current | The browser tests use the installed Chrome. Do not run `playwright install`. |

Fork the repository, then:

```sh
git clone https://github.com/<you>/monita.git
cd monita
make setup                             # git hooks, and the web and e2e packages
MONITA_RESET_TOKEN=dev make run        # Monita on http://127.0.0.1:7466
```

Load the sample accounts with the `reset-state` call in [Getting started](docs/guide/getting-started.md). [seeds/README.md](seeds/README.md) lists the accounts.

## Make a change

- Read [CLAUDE.md](CLAUDE.md) first. It states the rules every change follows: which module owns which table, where SQL and business rules live, the 400-line file cap, and the list limits. The docs in `docs/` win when CLAUDE.md and a doc disagree.
- Keep one change per pull request: one story, one fix or one doc update.
- A change in behaviour comes with a test at the module's interface: an integration test in `tests/`, a unit test beside pure logic, or a `bun test` beside a rule in `web/src/lib/`. A bug fix comes with a test that fails without the fix.
- A database change is a new file `migrations/000N_<what>.sql` plus one line in `src/platform/migrate.rs`. Never edit a migration that is already on `main`.
- A change to a route updates its contract in `docs/api-specs/` and its row in the tracker in `docs/api-specs/_index.md`.
- A change that users see updates the guide in `docs/guide/` and adds a line under `## [Unreleased]` in [CHANGELOG.md](CHANGELOG.md).
- Use the words in [docs/GLOSSARY.md](docs/GLOSSARY.md): "tech lead", not "TL".

Commit messages follow `type(scope): subject`, for example `fix(checks): retry a timed-out probe once`. The types are `feat`, `fix`, `docs`, `test`, `refactor`, `chore` and `perf`.

## Checks

```sh
make check      # formatting, clippy, Rust unit tests, web type-check and unit tests, Rust integration tests
make e2e        # the browser tests on port 18080; run them when a page or an API changed
```

The pre-commit hook runs `cargo fmt --check` and `clippy` when a commit touches Rust. To run one browser spec, use `cd e2e && bun run test specs/<name>.spec.ts --workers=2`. A new spec file must belong to a shard in `e2e/lib/shards.ts`.

CI runs `make check`, `cargo deny`, the browser tests in three shards and a `monita service` smoke test on a real systemd host. A change to `docs/` or Markdown alone skips the browser tests, and any job whose inputs passed before skips itself.

## Open a pull request

1. Create a branch from `main`, for example `fix/heartbeat-grace`.
2. Run `make check`, and `make e2e` when a page or an API changed.
3. Open the pull request and fill in the template.
4. A maintainer from [GOVERNANCE.md](GOVERNANCE.md) reviews it. CI must be green before it merges.

## Cut a release

Maintainers only.

1. Move the `## [Unreleased]` lines in [CHANGELOG.md](CHANGELOG.md) under a new version heading with the date.
2. Set the same version in `Cargo.toml` and `web/package.json`, then run `cargo build` to update `Cargo.lock`.
3. Commit as `chore: release vX.Y.Z` and tag that commit: `git tag -a vX.Y.Z -m vX.Y.Z`.
4. Push `main`, then the tag. `.github/workflows/release.yml` checks that the tag matches `Cargo.toml`, builds the static Linux binary, boots it, and opens a draft release with the archive, its checksum and a build-provenance attestation.
5. Write the release notes on the draft, then publish it. `releases/latest/download/monita_linux_amd64.tar.gz` then serves the new archive.

## Report a problem

Use the issue forms: bug report, feature request or documentation. Search the open issues first.
