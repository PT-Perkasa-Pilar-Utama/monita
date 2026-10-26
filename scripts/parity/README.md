# Parity kit

The parity kit proves that a new Monita binary gives the same answers as an old one on the same database. Use it for a change that must not change behaviour, such as the move from Turso to SQLite.

## What it checks

| Check | Tool | Pass |
| --- | --- | --- |
| Every GET route, read as 6 callers (anonymous, super admin, tech lead, supervisor, read key, write key), with range, state and page variants | `run.ts` | 0 regressions |
| A write scenario that touches every module: projects, monitors, heartbeats and pings, incidents, maintenance, changelog, keys, secrets, alert cards, tickets and ref IDs, invites, avatar and icon blobs, two-factor, time zone, locations and the probe | `run.ts` | 0 regressions |
| Every row of every table after the scenario, plus `PRAGMA integrity_check` and `foreign_key_check` | `run.ts` | 0 regressions, `ok` |
| A database that the old binary wrote, opened by the new binary | `run.ts` (the fixture comes from the base binary) | starts, answers |
| Read and write speed on 3 million checks and 1 million pings, memory, binary size | `perf.ts` | no measure **slower** |
| Unit and integration tests | `cargo test --lib --tests --no-fail-fast` | the same tests pass as on the base |
| The browser suite | `cd e2e && bunx playwright test --workers 2` | the same tests pass as on the base |

## How it compares

`run.ts` builds a fixture with the base binary: the demo seed, a read key, a write key and a secret, with every check, deadline and outbox retry moved far out. It then does three runs, each on its own copy of the fixture: base, new, base again. A value that differs between the two base runs is noise, such as a clock or a random token. Noise is still compared by type, so a number that comes back as text or null is caught. A value where the new run differs from both base runs is a regression. Differences that the change is meant to make are listed by name in `EXPECTED` in `run.ts`.

## Run it

Build the base binary from the commit before the change, and keep a copy outside `target/`:

```sh
git stash   # or check out the base commit
cargo build && cp target/debug/monita /tmp/monita-base
cargo build --release && cp target/release/monita /tmp/monita-base-release
git stash pop
cargo build && cargo build --release

bun scripts/parity/run.ts --base /tmp/monita-base --new target/debug/monita --out target/parity
bun scripts/parity/perf.ts --base /tmp/monita-base-release --new target/release/monita --fixture target/parity/fixture
```

`run.ts` takes about 5 minutes. It writes `report.md` and, for each run, `capture.json` (every answer) and `dump.json` (every row). It exits 1 on a regression. `perf.ts` needs release builds and about 1.5 GB of free disk.

## Test the kit itself

- **Self-test:** run `run.ts` with the same binary as base and new. It must report 0 regressions.
- **Planted regression:** change one value in `<out>/new/capture.json` or `<out>/new/dump.json`, then run `bun scripts/parity/run.ts --compare-only --out <out>`. It must report that value.

## What it does not prove

- **Behaviour over weeks:** a WAL file that grows, or a busy disk.
- **Every build target:** the musl and Windows builds. The release workflow's `build_only` dispatch builds and boots those.
- **Network time:** checks of real hosts. The kit's monitors point at a closed local port, so every run sees the same failure.
