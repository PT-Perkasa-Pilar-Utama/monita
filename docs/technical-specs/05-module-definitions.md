# 5. Module Definitions

Each module owns its tables (see [06](06-data-model.md)). Other modules use only the `pub` items in its `mod.rs`.

## 5.1 accounts

**Responsibility:** users, invites, sessions. Serves US-01 to US-04.

**Interface:**

| Item | Contract |
| --- | --- |
| `CurrentUser` extractor | Reads the `monita_session` cookie, looks up the hash in `sessions`, rejects expired or disabled users. Redirects to `/login` for HTML, returns 401 for JSON. |
| `require_role(Role)` | Returns 403 when the role does not match. |
| `create_super_admin(email, password)` | CLI only. Errors: `EmailTaken`. |
| `invite_tl(email) -> InviteLink` | Creates the user with no password, creates an invite, deletes the user's older invites and all sessions. Sends the email through `notify::send`. |

**Why deep:** callers never see hashes, expiry or cookie rules.

## 5.2 projects

**Responsibility:** projects, ownership, API keys, IP allowlist. Serves US-05 to US-08.

| Item | Contract |
| --- | --- |
| `ApiProject` extractor | Reads `Authorization: Bearer mk_...`, hashes it, finds a live key, checks the allowlist against the client IP. 401 for a missing or bad key, 403 for an IP outside the list. Updates `last_used_at` at most once per minute. |
| `owner_of(project_id) -> UserId` | Used by `notify` and access checks. |
| `can_manage(user, project_id) -> bool` | True for the owner and for every super admin. |

## 5.3 checks

**Responsibility:** monitors, heartbeats, pings, results, summaries, the scheduler loop, retention. Serves US-09 to US-14 and US-32.

| Item | Contract |
| --- | --- |
| `probe(&Monitor, &Client) -> CheckResult` | One HTTP request with the monitor timeout. `ok` when the status matches `expected_status` and the body contains `keyword` (if set). Never returns an error: failures are a `CheckResult` with `ok = false` and an `error` string. Reads at most 1 MB of body. |
| `record_ping(token, kind)` | Used by the `/ping` routes. Unknown token: `NotFound`. Pushes `deadline_at` forward. |
| scheduler loop | Internal. See [13](13-check-scheduler.md). |

**Seam:** the outbound HTTP client is a `reqwest::Client` in `AppState`. Tests point monitors at a local axum test server, so no port is needed.

## 5.4 incidents

**Responsibility:** the incident lifecycle and its timeline. Serves US-17 to US-20.

| Item | Contract |
| --- | --- |
| `observe(source, signal) -> Option<Event>` | The only door for check results. Applies the 2-fail rule, opens and closes automatic incidents, writes the system status updates, and returns the event for `notify`. Idempotent for a repeated signal. See [12](12-incident-engine.md). |
| `open_manual(project, items, message, author)` | Creates a manual incident with the first update, label Investigating. |
| `post_update(incident, label, message, author)` | Message is required. Resolved on a manual incident closes it. |
| `current_state(source) -> ItemState` | `Up`, `Down`, `Issue` (open manual incident) or `Unknown`. Used by `status`. |

**Why deep:** checks and the UI know nothing about streaks or the open and close rules. `engine.rs` holds them as pure functions.

## 5.5 notify

**Responsibility:** alert routes, the outbox, delivery, the ticket digest. Serves US-15, US-16 and US-30.

| Item | Contract |
| --- | --- |
| `send(Event)` | Looks up routes for the project and severity, inserts one outbox row per recipient, and returns. It never blocks on SMTP or HTTP. |
| outbox loop | Internal. Claims due rows in a write transaction (a 120 s lease on `next_attempt_at`), so two passes never send one row twice. Retries at 1 min, 5 min and 15 min, then marks the row failed and logs it. Delivery is at least once: a crash mid-send retries after the lease. |

**Ports:** `Mailer` is an enum: `Smtp` (lettre, OS trust store), `Log` (no `MONITA_SMTP_URL`), `Memory` (tests). Webhooks go through the strict reqwest client; tests point them at a local server, so no webhook port exists.

**Webhook body:** `{"text": "...", "event": "...", "project": "...", "severity": "...", "incident_url": "..."}`. Slack and Teams incoming webhooks read `text`.

## 5.6 status

**Responsibility:** the public status page and JSON. Serves US-21 and US-22. Owns no tables. Reads through `checks` (public items, daily summaries) and `incidents` (current state, history).

| Route | Output |
| --- | --- |
| `GET /status/{slug}` | HTML |
| `GET /api/v1/public/status/{slug}` | JSON with the same data |

## 5.7 tickets

**Responsibility:** ref IDs, tickets, kanban, the public board, the reporter view. Serves US-23 to US-29. Calls `notify::send(Event::TicketCreated)` for the webhook. The digest reads tickets through `tickets::new_in_triage_since(project, t)`.

| Item | Contract |
| --- | --- |
| `issue_ref_id(project, single_use) -> String` | Returns the raw ID once. Stores its hash. |
| `submit(ref_id, fields)` | Checks the ref ID (known, not revoked, not used up), the 5 per ref ID per day limit and the 100 per project per day limit. Marks a single-use ID as used in the same transaction. |

## 5.8 platform

**Responsibility:** configuration, database handle, migrations, rendering, assets, settings, rate limits, health, backup, peer ping, reset-state.

### 5.8.1 Health monitor

`GET /healthz`, public.

```json
{"status": "ok", "db": "ok", "scheduler_lag_s": 1, "outbox_pending": 0, "version": "1.0.0"}
```

| Condition | HTTP | `status` |
| --- | --- | --- |
| `SELECT 1` works and the scheduler ticked in the last 30 s | 200 | `ok` |
| The DB query fails | 503 | `fail`, `db: "fail"` |
| The scheduler has not ticked for 30 s | 503 | `fail` |

The body never holds paths, versions of dependencies or secrets.

### 5.8.2 Reset state

`POST /admin/reset-state`, body `{"seed": "dev" | "qa"}`.

- Registered only when `MONITA_ENV` is `dev` or `test`. In `prod` the route is never added to the router, so it returns 404.
- Needs a super admin session, except in `test`, where it needs the header `X-Reset-Token` equal to `MONITA_RESET_TOKEN`. The e2e suite uses this.
- Runs synchronously: deletes every row in every table, re-runs migrations, loads `seeds/<seed>.sql`. Returns 204.

### 5.8.3 Peer ping

When `MONITA_PING_URL` is set, a loop sends `GET` to it every 60 s. A failure logs a warning. It never raises an incident.

### 5.8.4 Backup

Daily at 01:00 UTC. Writes `monita-YYYYMMDD.db` to `MONITA_BACKUP_DIR` with `VACUUM INTO` through the writer connection, and keeps the newest `MONITA_BACKUP_KEEP` files. The spike on 2026-10-16 confirmed `VACUUM INTO` on turso 0.8.3; `sqlite3` reads the copy with a clean `integrity_check`.

### 5.8.5 Database access

| Item | Contract |
| --- | --- |
| `Db::write(f)` | Locks the single writer connection (`tokio::sync::Mutex<turso::Connection>`), runs `f` inside `BEGIN IMMEDIATE ... COMMIT`, rolls back on error. Every write in Monita goes through it. |
| `Db::read()` | Returns a new `turso::Connection` for reads. |
| Every connection | `PRAGMA busy_timeout = 5000` and `PRAGMA foreign_keys = ON` at open. |

Spike result (50 tasks, 2 000 inserts): one writer behind a mutex took 75 ms; 50 connections with `busy_timeout` took 834 ms; 50 connections without it hit 161 000 busy errors. Rule: no module opens its own writing connection.
