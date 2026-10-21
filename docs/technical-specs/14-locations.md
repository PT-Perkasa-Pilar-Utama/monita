# 14. Locations: checks from more than one place

Decision: [v1.1 Q26](../decisions/2026-10-19-v1.1-scope.md). Issue #6. Owner module: `checks`.

## 14.1 Purpose

Every check ran from the one server that runs Monita, so Monita could not tell a regional outage from its own network failing. A location is another machine that runs checks for this Monita and reports the results. A monitor checks from this server and from the locations a tech lead picks. It counts a check as failed only when enough of those places agree.

## 14.2 Parts

```text
 this server                                   a location (another machine)
 ┌───────────────────────────┐   HTTPS          ┌──────────────────────────┐
 │ scheduler: local checks   │ <── GET work ──  │ monita probe             │
 │ /api/v1/probe/work        │ ── jobs ──────>  │  runs the same probe code│
 │ /api/v1/probe/results     │ <── POST ─────── │  posts the results       │
 │ verdict -> incidents      │                  └──────────────────────────┘
 └───────────────────────────┘
```

- `monita probe` is a subcommand of the same binary. It needs no database and no config file.
- The probe pulls work. The server never connects to a location, so a location behind NAT works.

## 14.3 Data

Migration 0030.

| Table | Column | Type | Notes |
| --- | --- | --- | --- |
| `locations` | `id` | INTEGER PK AUTOINCREMENT | |
| | `name` | TEXT NOT NULL UNIQUE | 1 to 40 characters, such as `Jakarta` |
| | `token_hash` | TEXT NOT NULL UNIQUE | SHA-256 of the token. The token is shown once. |
| | `token_prefix` | TEXT NOT NULL | The first 4 characters after `ml_`, to tell tokens apart |
| | `last_seen_at` | INTEGER | The last call from the probe; null until the first |
| | `created_at` | INTEGER NOT NULL | |
| `monitor_locations` | `monitor_id` | INTEGER NOT NULL, FK monitors ON DELETE CASCADE | |
| | `location_id` | INTEGER NOT NULL, FK locations ON DELETE CASCADE | |
| | `next_check_at` | INTEGER NOT NULL | When this location checks this monitor next |
| | | PRIMARY KEY (`monitor_id`, `location_id`) | index on (`location_id`, `next_check_at`) |
| `monitors` | `quorum` | INTEGER NOT NULL DEFAULT 1, `BETWEEN 1 AND 11` | How many places must fail |
| `check_results` | `location_id` | INTEGER, FK locations ON DELETE CASCADE | Null: this server. Index (`monitor_id`, `location_id`, `checked_at`) |

`ON DELETE CASCADE` on `check_results.location_id`: deleting a location deletes its results. `SET NULL` would turn them into results of this server.

Caps (`platform::caps`): 10 locations; 50 jobs in one work answer; 50 results in one post.

## 14.4 Token

- Format `ml_` + 26 base32 characters (`token::random_token`), like an API key's `mk_`.
- Only the SHA-256 hash is stored (SEC-07). The token is never logged.
- The probe reads it from `MONITA_PROBE_TOKEN`; `--token` is the fallback, because a command line shows in `ps`.
- A request sends it as `Authorization: Bearer ml_…`.
- A wrong token counts in the caller IP's `keymiss:` bucket, as a wrong API key does, and answers 401.
- A known token counts in its own bucket `probe:{id}`: 300 calls a minute. A probe that polls each second uses 60.

**Trust boundary.** A location token holder receives every job of that location, and a job carries the monitor's secret value when the monitor signs in. A location token is therefore as sensitive as every project secret its monitors use. Only a super admin creates or deletes a location. A supervisor reads the locations page; there is no token to read after creation.

## 14.5 Work

`GET /api/v1/probe/work?free=N`, N from 1 to 50 (missing: 50).

1. Authenticate the token (14.4).
2. On a read connection: does this location have a due row (`next_check_at <= now`)? If not, answer `{"jobs": []}`. Update `last_seen_at` only when it is 30 s or more old, so an idle probe does not take the writer each second.
3. Otherwise, in one write: claim up to N due rows by setting `next_check_at = now + interval_s`, `claimed_at = now` and `open_claims = min(open_claims + 1, 2)` (migration 0031), set `last_seen_at = now`, and read the jobs.
4. A job is everything one check needs:

```json
{ "monitor_id": 12, "kind": "http", "target": "https://pay.example.com/healthz",
  "method": "GET", "headers": "Accept: application/json", "body": null,
  "timeout_ms": 10000, "expected_status": "2xx", "keyword": null, "skip_tls_verify": false,
  "expected_address": null, "secret_gone": false,
  "auth": { "kind": "bearer", "user": null, "header": null, "secret": "…" } }
```

`auth` is null when the monitor does not sign in. When its secret was deleted, `auth` is null and `secret_gone` is true, and the job fails with the fixed sentence `SECRET_GONE`, as a local check does.

## 14.6 Results

`POST /api/v1/probe/results` with `{"results": [{"monitor_id", "ok", "status_code", "latency_ms", "error"}]}`, 1 to 50 items.

- The server takes `checked_at` from its own clock on receipt. A location's clock may drift.
- A result must answer a claim. Each claim adds one to the row's `open_claims` (at most 2, so a slow probe handed the same check again loses neither result) and sets `claimed_at`; each stored result takes one away. A result is refused when no claim is open, or when the newest claim is older than two intervals, the timeout and 30 seconds, since it would count as fresh while it is not. Refused results are counted in `skipped`. A probe therefore stores at most one row per check it was handed.
- `error` is cut to 1 KB.
- The whole batch is one `Db::write`. Spending the claim inside it also skips a monitor deleted since the claim, so one gone monitor never fails the batch.
- A location's result feeds no verdict by itself. It counts in the next verdict of a check from this server (14.7).
- The answer is `{"stored": n, "skipped": m}`.

## 14.7 Verdict

A pure function, `checks::verdict::judge`, decides what a check from this server means for the monitor, given every place's newest result. It runs once per check from this server; a location's result only waits to be counted.

Inputs:

- For each place the monitor checks from (this server, and each assigned location): its newest result, or none.
- `interval_s`, `quorum`, and now.

Rules:

1. A result is fresh when `checked_at >= now - (2 × interval_s + 30)`.
2. `fresh` is the number of places with a fresh result. `failing` is how many of those failed.
3. `needed = min(quorum, fresh)`.
4. The check counts as failed when `failing >= needed` and `failing >= 1`. Otherwise it counts as passed.
5. The failure's cause names each failing place: one place keeps its own error text; several read `2 of 3 places failed: This server: Timeout after 10000 ms; Jakarta: HTTP 503`.

| Places | quorum | Fresh results | Verdict |
| --- | --- | --- | --- |
| This server only | 1 | fail | failed (as before 1.1) |
| This server, Jakarta, Tokyo | 2 | fail, ok, ok | passed |
| This server, Jakarta, Tokyo | 2 | fail, fail, ok | failed |
| This server, Jakarta, Tokyo | 2 | fail; the others stale | failed (`needed` = 1) |
| This server, Jakarta | 2 | ok, fail | passed |

The verdict feeds `incidents::observe` as `CheckOk` or `CheckFailed`. There is one verdict per check from this server, so the failed-check count in a row (`fails_to_open`) keeps its meaning, "N intervals in a row", whatever the number of places. A monitor with no locations reads its own result, the same as before 1.1, with no query: the monitor row carries its location count. A failure that only a location sees is raised at this server's next check, at most one interval later.

Rule 3 means a location that goes silent never blocks an incident: the places that still answer decide.

## 14.8 Pages

- **Locations** (`/admin/locations`): super admin manages, supervisor reads. Each row: name, token prefix, last seen ("Never", "12s ago"), monitors that use it. Add shows the token once with the probe command. Delete asks first and removes the location's results.
- **Monitor form**: when locations exist, **Check from** lists this server (always on) and each location, and **Counts as failed when** `[quorum]` of the places fail.
- **Monitor page**: when the monitor has locations, a **Places** card lists each place's newest result and how old it is. The check list names the place of each check.
- Charts, the 24-hour sparkline, daily summaries, the monitor list's last check and the page header's state use this server's results only, so response times stay comparable. A monitor that only a location has checked so far reads "No data yet" until this server's first check.

## 14.9 Not in 1.1

| What | Why |
| --- | --- |
| An alert when a location goes silent | The Locations page shows Last seen. A probe's host can ping a Monita heartbeat to get alerts today. |
| Charts per location | The Places card shows each place's newest result. |
| `monita service install --probe` | The guide gives a systemd unit for the probe. |
| Monitors that skip this server | This server is always a place, so a lost probe never leaves a monitor unchecked. |
