# 5. Module Definitions

Each module owns its tables (see [06](06-data-model.md)). Other modules use only the `pub` items in its `mod.rs`.

## 5.1 accounts

**Responsibility:** users, invites, sessions, roles. Serves US-01 to US-04. Roles are `super_admin`, `tl` and `supervisor` ([09](09-authentication-and-authorization.md)).

**Interface:**

| Item | Contract |
| --- | --- |
| `CurrentUser` extractor | Reads the `monita_session` cookie, looks up the hash in `sessions`, rejects expired or disabled users. Returns 401 under `/api/`; any other path redirects to `/login`. A supervisor gets 403 on any method but `GET` and `HEAD`. Every signed-in route that changes data takes this extractor, so this one check keeps the role read-only. |
| `AdminUser` extractor | `CurrentUser` that sees every project: a super admin, or a supervisor reading. Others get 403. A supervisor's writes never get this far. |
| `User::is_admin`, `sees_all`, `is_read_only` | Super admin only; super admin or supervisor; supervisor only. Handlers branch on these, never on the role string. |
| `is_active_tl(c, user_id) -> bool` | Used by `projects::reassign`. |
| `purge_expired(c, now, limit)` | Retention: expired sessions and invites, and invites used more than 7 days ago. The caller loops. |
| `create_super_admin(email, password)` | CLI only. Errors: `EmailTaken`. |
| `invite(db, base_url, email, role) -> link` | Creates the user with no password and the given role, creates an invite and queues the email through `notify::queue_email`. `invite_tl` is the `Tl` case. Errors: `EmailTaken`, `InvalidEmail`, `Full` (500 accounts). |
| `reinvite(db, base_url, user_id)` | A new invite for an active user. Deletes the user's older invites and all their sessions. |
| `set_role(db, actor, user_id, role)` | Promotes or demotes an active user. Never your own account. Refuses `Supervisor` for a project owner (`AdminError::OwnsProjects`, checked with `projects::owns_any` inside the write), because an owner must be able to change the project. |
| `disable(db, actor, user_id)`, `enable(db, user_id)`, `reset_sign_in(db, base_url, actor, user_id)` | Disable signs the user out and drops their invites. Enable undoes it. `disable` and `reset_sign_in` refuse your own account (`AdminError::Yourself`). |

**Why deep:** callers never see hashes, expiry or cookie rules.

## 5.2 projects

**Responsibility:** projects, their description, homepage and icon, ownership, API keys, IP allowlist, the public tab switches, sealed secrets. Serves US-05 to US-08.

| Item | Contract |
| --- | --- |
| `ApiProject` extractor | Reads `Authorization: Bearer mk_...`, hashes it, finds a live key, checks the allowlist against the client IP. 401 for a missing or bad key, 403 for an IP outside the list. Updates `last_used_at` at most once per minute. |
| `find_for(db, user, slug) -> Project` | The project a user may open, or 404 so other projects' names do not leak. |
| `can_view(user, project) -> bool` | True for the owner, every super admin and every supervisor. A supervisor's writes are refused earlier, in `CurrentUser`. |
| `tab_counts(db, project_id)` | The numbers beside the project tabs, each read through its module. |
| `create(db, acting, input)` | US-06. Makes the slug from the name. A TL owns what they create; a super admin owns it unless `owner_email` names an eligible owner. `homepage_url` is required. An unknown owner, a missing or bad homepage, a description over 280 characters or a bad icon is a form error, and `ProjectError::Invalid` names every bad field at once. |
| `eligible_owners(db, acting)` | The owners a super admin may pick: the acting super admin first, then active TLs. |
| `find_owner(db, acting, email)` | The eligible owner with that email, case ignored, or none. |
| `reassign(db, project_id, new_owner, acting)` | US-05. The new owner is an active TL or the acting super admin; anyone else is 404. |
| `update_profile(db, project_id, input)` | Saves the description and the homepage (required, as for `create`) and sets, keeps or removes the icon. |
| `page_for(db, user, q, owner, sort, page)` | One page (`caps::PROJECTS_PAGE`, 20) of the projects a user may open, with the total. A TL pages their own; a super admin or a supervisor pages every project. `q` matches name, slug, owner email and description. |
| `pulse(...)` | The project list's row figures: the worst state, the monitor and heartbeat summaries, open incidents and tickets, and 24 hourly bars from automatic incidents. ponytail: the list reads each project's pulse one by one (20 at most a page); batch the reads if the page gets slow. |
| `delete(db, project, confirm)` | Super admin only. `confirm` must equal the project's name. Inside one write: `checks::forget_project` drops the daily summaries, `incidents::forget_sources` drops the fail streaks, then the project and its children cascade. No alert goes out. |
| `set_pages(db, project_id, PublicPages)` | The public tab switches: Incidents, Known issues, Report a bug, Changelog. Uptime always shows. A missing `incidents` means on; a missing `changelog` means off. |
| `owns_any(c, user_id)` | True when the user owns a project. `accounts::set_role` calls it inside its own write. |
| `secrets::{list, create, replace, delete}` | Sealed secrets per project. `list` returns names with `used_by` (how many monitors sign in with each), never values. `delete` is refused while a monitor uses the secret. Caps: 20 secrets per project, 4 KB a value. |
| `secrets::{exists, name_of, value_of}` | `exists` and `name_of` serve the monitor form and view. `value_of` is the only door to a value; `checks::run_check` is its only caller (SEC-23). |

## 5.3 checks

**Responsibility:** monitors, heartbeats, pings, results, summaries, the scheduler loop, retention. Serves US-09 to US-14 and US-32.

| Item | Contract |
| --- | --- |
| `probe(&Target, &Clients) -> CheckResult` | One HTTP request with the monitor's method, headers, body and timeout, and its credential when `Target.auth` is set. `ok` when the status matches `expected_status` and the body contains `keyword` (if set). Never returns an error: failures are a `CheckResult` with `ok = false` and an `error` string. Reads at most 1 MB of body. Values of headers, the body and the credential never appear in the error. |
| `run_check(db, &Clients, &Monitor) -> Result<CheckResult>` | What the scheduler calls. For a monitor with `auth_kind` other than `none` it reads the secret just before the probe, through `projects::secrets::value_of`. A deleted secret gives a failed check with the fixed sentence `SECRET_GONE`. `Err` only when the secret cannot be read; the check is skipped then, not failed. |
| `create_monitor`, `update_monitor` -> `Saved` | `Saved::Done(id)`, `Full` (100 monitors), `NotFound` or `ForeignSecret` (the form names a secret of another project, a 422 on `auth_secret_id`). Both pick a random first check inside the interval. |
| `delete_monitor(db, base_url, project_id, id)` | Also closes the monitor's open incident and alerts that it ended. |
| `rearm_heartbeat(c, id, now)` | Sets `deadline_at = now + period_s + grace_s`. Called by `incidents::post_update` when a TL resolves a heartbeat's incident by hand. |
| `alert_on(c, kind, id)`, `secret_users(c, id)`, `source_names(c, project_id)` | Reads other modules call inside their own write: the source's alert switch, how many monitors use a secret, the project's monitors and heartbeats by name. |
| `forget_project(c, project_id)` | Drops the daily summaries of the project's sources, which have no foreign key, and returns the sources. `projects::delete` calls it. |
| `counts(db, project_id)`, `summaries(db, project_id)` | The project's monitor and heartbeat totals and state lines, for the About column and the project list. |
| `admit(db, limiter, &Ping) -> Admission` | Used by the `/ping` routes first. Refuses an IP over its unknown-token budget, finds the heartbeat by token (`UnknownToken`), then applies the per-heartbeat limit (`Limited(retry)`, with `/fail` in its own bucket). Returns an `Admitted` value that only `record` takes; a ping body is read only after it. |
| `record(db, base_url, Admitted, cause) -> bool` | Stores the ping: an OK ping pushes `deadline_at` forward, a fail ping opens an incident. False when the heartbeat was deleted after `admit`. |
| scheduler loop | Internal. See [13](13-check-scheduler.md). |

**Seam:** the outbound HTTP client is a `reqwest::Client` in `AppState`. Tests point monitors at a local axum test server, so no port is needed.

## 5.4 incidents

**Responsibility:** the incident lifecycle and its timeline. Serves US-17 to US-20.

| Item | Contract |
| --- | --- |
| `observe(c, &Source, &Signal, now) -> Option<Event>` | The only door for check results. Applies the source's `fails_to_open` rule, opens and closes automatic incidents, writes the system status updates, and returns the event for `notify`. With `auto_incident` off it returns `Down` and `Up` events and opens nothing; with `alert` off it returns no event. While an active maintenance window covers the source, it opens nothing and returns no event (12.3a). Idempotent for a repeated signal. See [12](12-incident-engine.md). |
| `open_manual(db, &NewIncident, now)` | Creates a manual incident (title, items, severity, first message, author) with the first update, label Investigating. `items` are the form's `kind:id` values; it keeps those that name one of the project's monitors or heartbeats, read through `checks`, once each. Errors: empty title, no items, empty message. |
| `post_update(db, base_url, project_id, incident_id, label, message, author, now)` | Message is required (1 to 10 000 characters), and an incident takes 50 updates. Resolved on a manual incident closes it. Resolved on an open automatic incident also closes it, by hand: see below. A closed incident takes only Resolved, as a follow-up note (AC-19.03). `allowed_labels(origin, closed)` gives the form the same list. |
| `states_for(db, project_id) -> HashMap<(SourceKind, id), ItemState>` | `Down` while an automatic incident is open, `Issue` while a manual one names the item, `Maintenance` while an active window covers an item no incident names. Items with no entry are up, or waiting before their first check. The public status page uses it, so only an incident someone opened reaches visitors. |
| `team_states_for(db, project_id)` | `states_for`, plus `Down` for a source that is down with no incident because its incident switch is off (`incident_sources.down_since` set). Used by the team's pages. |
| `source_deleted(c, &Source, now)`, `forget_sources(c, sources)` | A deleted source closes its open automatic incident and returns the `Resolved` event. A deleted project forgets its sources' streaks and sends nothing. |
| `across(db, owner, &AcrossFilter, page)` | One page (`caps::ACROSS_PAGE`, 25) of incidents across projects, with the total. `owner` of `None` reads every project. Search matches the title, `INC-n` or the project's name. `state` is `open` (the default), `resolved` or `all`. The default order is critical first, then newest; `sort` may be `newest` or `oldest`. |
| `open_count(db, project_id)` | Open incidents, for the project's tab number. |

**Hand resolve.** A TL's Resolved on an open automatic incident closes it, writes the TL's update, and sends the resolved alert ("marked resolved by a tech lead", `Ended::ByHand`) when the source's alerts are on. It clears the source's streak, open incident and `down_since`, so a monitor needs `fails_to_open` new failed checks to open the next incident. For a heartbeat it also calls `checks::rearm_heartbeat`, so a job that stays silent opens the next incident one period plus grace later. No system "Recovered" entry is written: the TL's update is the close.

**Why deep:** checks and the UI know nothing about streaks or the open and close rules. `engine.rs` holds them as pure functions.

## 5.5 notify

**Responsibility:** alert routes, ticket alerts, the outbox, delivery, the mail port, the ticket digest. Serves US-15, US-16 and US-30. A project whose owner is disabled gets no digest; its Triage tickets wait on the ticket board for the new owner, and the digest watermark moves past them.

| Item | Contract |
| --- | --- |
| `send(c, &Event, base_url)` | Looks up the enabled route for the event's severity, inserts one outbox row per recipient, and returns. It never blocks on SMTP or HTTP. It handles `Opened`, `Resolved`, `Down` and `Up`. |
| `previews(name, slug, base_url, at) -> Vec<Preview>` | A sample of every kind of alert, built by the code that builds the real ones: the event, label, route (`critical`, `warning` or `ticket`), the email and the webhook body. `GET /api/ui/projects/{slug}/alerts-preview` serves it. |
| `routes`, `save_route`, `set_enabled` | The Critical and Warning routes: emails (at most 20), webhook, and the on and off switch. Off keeps the recipients. |
| `ticket_alerts`, `save_ticket_alerts` | The Ticket alerts section: digest, the ticket webhook URL, the new-ticket box (`use_webhook`) and the moves box (`webhook_moves`). A box on needs the URL. |
| `send_ticket(c, project_id, number, title, base_url)` | One `ticket.created` call for a reporter's ticket, when the ticket webhook is on. Inside the submit transaction. |
| `send_ticket_moved(c, project_id, &Moved, base_url)` | One `ticket.moved` call (`from`, `to`, `close_reason`) when the moves box is on. A move to the same column sends nothing. |
| `send_test`, `send_ticket_test` | The test buttons. A test sends the `test` event, which never looks like a ticket. The ticket test works when either webhook box is on. |
| `test_connection(&MailSettings)`, `Mailer::send_now` | The admin Email page: connect and sign in without sending, and send one email now to any address. The error says what failed. |
| `email_health(db, since)` | Sent, failed and waiting counts and the newest error, for the Email page. |
| outbox loop | Internal. Claims due rows in a write transaction (a 120 s lease on `next_attempt_at`), so two passes never send one row twice. Retries at 1 min, 5 min and 15 min, then marks the row failed and logs it. Delivery is at least once: a crash mid-send retries after the lease. Bodies are cleared after delivery or final failure, so an invite link never stays in the database. Without SMTP the log shows recipient and subject only. |

**Ports:** `Mailer` is an enum: `Live` and `Memory` (tests). `Live` picks its server on each send, so a save on the Email page applies to the next email: the mail server a super admin saved (`settings.mail`), else `MONITA_SMTP_URL` (built once at start, lettre, OS trust store), else the log. The log shows recipient and subject only. Webhooks go through the strict reqwest client, which follows no redirect; tests point them at a local server, so no webhook port exists.

**Webhook body:** every body has `text`, `event`, `project` and `at`. Slack and Teams incoming webhooks read `text`. The rest depends on the event.

| `event` | Extra fields | Sent when |
| --- | --- | --- |
| `incident.opened` | `source`, `severity`, `cause`, `incident_url` | An automatic incident opens |
| `incident.resolved` | `source`, `severity`, `duration_s`, `incident_url` | An automatic incident ends: a passing check, a deleted source or a hand resolve |
| `source.down` | `source`, `severity`, `cause`, `source_url` | A source with its incident switch off goes down. The text starts `[DOWN]`. |
| `source.up` | `source`, `severity`, `duration_s`, `source_url` | That source works again. The text starts `[UP]`. |
| `ticket.created` | `ticket_number`, `title`, `ticket_url` | A reporter files a ticket and the new-ticket box is on |
| `ticket.moved` | `ticket_number`, `title`, `from`, `to`, `close_reason`, `ticket_url` | A ticket changes column and the moves box is on |
| `test` | `severity` (alert routes only), `link` (ticket alerts only) | A test button |

Critical and warning events go to the route of the event's severity. Ticket events go to the ticket webhook. The warning webhook no longer hears tickets. The digest skips a project whose Ticket alerts turn the digest off.

## 5.6 status

**Responsibility:** the public status page and JSON. Serves US-21 and US-22. Owns no tables. Reads through `checks` (public items, daily summaries) and `incidents` (current state, history).

| Route | Output |
| --- | --- |
| `GET /api/ui/public/{slug}/status` and `/history` | JSON for the React status pages. `/history` answers 404 while the Incidents tab is off. |
| `GET /api/ui/public/{slug}/incidents?to=` | Three months of incidents at a time, 25 a month. 404 while the Incidents tab is off. |
| `GET /api/ui/public/{slug}/uptime?kind=&id=&to=` | Three months of one public system, as day grids with each month's uptime. 404 for a system that is not public. |
| `GET /api/v1/public/status/{slug}` | JSON with the same data as `/status`. With the Incidents tab off it carries no incidents. |

The page JSON carries the tab switches (`pages`). A public route whose tab is off answers 404. Availability counts automatic incidents only. The Uptime tab reads the page again every 60 s.

## 5.7 tickets

**Responsibility:** ref IDs, tickets, the ticket board, the public board, the reporter view. Serves US-23 to US-29. Calls `notify::send_ticket` inside the submit transaction and `notify::send_ticket_moved` inside the move transaction. The digest reads tickets through `tickets::new_in_triage_after(project, ticket_id)`. Ticket numbers are `max + 1` per project inside the writer transaction. The public board filters Triage in SQL and again in its route, count included (SEC-14, two layers on purpose).

| Item | Contract |
| --- | --- |
| `issue_ref_id(db, project_id, single_use) -> (raw, created_at)` | Returns the raw ID once. Stores its hash and its last 4 characters (`tail`, SEC-26). The board's "Issue a ref ID" dialog and `POST /api/v1/ref-ids` both call it. |
| `submit(db, base_url, only_project, ref_id, ticket)` | Checks the fields, then the ref ID (known, not revoked, not used up, and of `only_project` when the call comes from a project's own form), then the 5 per ref ID per day limit and the 100 per project per day limit. Marks a single-use ID as used and calls the new-ticket webhook in the same transaction. |
| `file(db, project_id, user_id, stage, ticket)` | A team member files a ticket into Triage, Backlog or In progress from the board. Sets `created_by`, no ref ID. No daily caps and no webhook: those guard against reporter traffic. |
| `column(db, project_id, stage, page)`, `public_column(...)` | One page (`caps::TICKETS_PER_COLUMN`, 25) of a column, newest change first. The public copy returns nothing for Triage. |
| `move_stage(db, base_url, project_id, number, stage, reason)` | Any column to any column. `done` needs a close reason; leaving it clears the reason. A move to another column calls `notify::send_ticket_moved` in the same write. |
| `set_public_title`, `revoke_ref` | The public title (an empty one clears it) and revoking the ref ID behind a ticket. |
| `triage_across(db, owner, q, sort, page)` | One page (`caps::ACROSS_PAGE`) of Triage tickets across projects. Search matches title, `#n` and project name. |
| `board`, `public_board`, `mine`, `stage_counts`, `open_count`, `new_in_triage_after` | Board reads, the reporter's tickets by ref ID, counts for tab numbers, and the digest's id watermark. |

## 5.8 platform

**Responsibility:** configuration, database handle, migrations, the JSON envelope, the React app and static files, settings, rate limits, health, backup, peer ping, reset-state.

### 5.8.1 Health monitor

`GET /healthz`, public.

```json
{"status": "ok", "db": "ok", "scheduler_lag_s": 1, "outbox_pending": 0, "version": "1.0.0"}
```

| Condition | HTTP | `status` |
| --- | --- | --- |
| The outbox count answers and the scheduler ticked in the last 30 s | 200 | `ok` |
| The DB query fails | 503 | `fail`, `db: "fail"` |
| The scheduler has not ticked for 30 s | 503 | `fail` |

`outbox_pending` counts outbox rows not yet sent or failed, retries included, so a mail outage shows as a growing number. The body never holds paths, versions of dependencies or secrets.

### 5.8.2 Reset state

`POST /admin/reset-state`, body `{"seed": "dev" | "qa" | "demo"}`. Contract: [api-specs 09 section 9.2](../api-specs/09-system.md).

- Registered only when `MONITA_ENV` is `dev` or `test`. In `prod` the route is never added to the router, so it returns 404.
- Needs the header `X-Reset-Token` equal to `MONITA_RESET_TOKEN`. The e2e suite and the demo snapshot use it.
- Runs synchronously: deletes every row in every table, re-runs migrations, loads `seeds/<seed>.sql`. Returns 204.

### 5.8.3 Peer ping

When `MONITA_PING_URL` is set, a loop sends `GET` to it every 60 s. A failure logs a warning. It never raises an incident.

### 5.8.4 Backup

Daily at 01:00 UTC. Writes `monita-YYYYMMDD.db` to `MONITA_BACKUP_DIR` with `VACUUM INTO` on its own reading connection, so writes go on during the copy, and keeps the newest `MONITA_BACKUP_KEEP` files. The spike on 2026-10-16 confirmed `VACUUM INTO` on turso 0.8.3; `sqlite3` reads the copy with a clean `integrity_check`.

### 5.8.5 Database access

| Item | Contract |
| --- | --- |
| `Db::write(f)` | Locks the single writer connection (`tokio::sync::Mutex<turso::Connection>`), runs `f` inside `BEGIN IMMEDIATE ... COMMIT`, rolls back on error. Every write in Monita goes through it. |
| `Db::read()` | Returns a new `turso::Connection` for reads. |
| Every connection | `PRAGMA busy_timeout = 5000` and `PRAGMA foreign_keys = ON` at open. |

Spike result (50 tasks, 2 000 inserts): one writer behind a mutex took 75 ms; 50 connections with `busy_timeout` took 834 ms; 50 connections without it hit 161 000 busy errors. Rule: no module opens its own writing connection.

### 5.8.6 Time zone

A super admin sets one fixed offset from UTC, in minutes, from -720 to 840 in quarter hours (`settings::utc_offset`, raw key `utc_offset_min`, default 0). `PUT /api/ui/admin/time-zone` saves it. `/api/ui/me`, `/api/ui/public/{slug}` and the System view carry `utc_offset_min`, and the web app formats every time with it. `platform::time` holds the helpers: `stamp`, `local`, `zone`, `day_at`, `day_start_at` and the month helpers. Storage stays UTC Unix seconds.

ponytail: a fixed offset with no daylight saving. Switch to named zones (a tz database) if a team in a DST country needs it. Daily summaries stay UTC days, as do the ticket daily caps, retention and the digest hour.

### 5.8.7 Mail server

A super admin saves the mail server on the Email page: `PUT` and `DELETE /api/ui/admin/mail`, `POST /api/ui/admin/mail/test-connection` and `POST /api/ui/admin/mail/test-send`. It lives in the raw settings key `mail` (JSON). The mailer reads it on each send, then falls back to `MONITA_SMTP_URL`, then to the log. The view names the source in use (`saved`, `env` or `none`) and says `has_password`; it never returns the password. A blank password in the form keeps the saved one; `remove_password` clears it. Test connection tries the form's values unsaved. Send a test email goes out at once, outside the outbox, to any address.

## 5.9 home

**Responsibility:** the signed-in pages above the projects: Incidents and Tickets across every project the user may open. Owns no tables. Every read goes through the `incidents` and `tickets` interfaces.

| Route | Output |
| --- | --- |
| `GET /api/ui/incidents` | One page of incidents across projects: `?q=&state=&severity=&sort=&page=`. 25 a page (`caps::ACROSS_PAGE`). Open incidents by default, critical first. |
| `GET /api/ui/tickets` | One page of Triage tickets across projects: `?q=&sort=&page=`. 25 a page. |

Scope: a TL sees their own projects. A super admin and a supervisor see every project. The module passes `owner = None` for them to `incidents::across` and `tickets::triage_across`.

Owner scope in SQL. Both stores add the same read-only clause, the one allowed `projects` read besides the name search ([CLAUDE.md](../../CLAUDE.md) architecture laws):

```sql
AND (?1 IS NULL OR project_id IN (SELECT id FROM projects WHERE owner_id = ?1))
```

The project tabs show numbers only for what needs a look: monitors and heartbeats down, open incidents and Triage tickets (`projects::tab_counts`). The header and the Changelog tab show none.

## 5.10 changelog

**Responsibility:** a project's releases, written by its team and shown on the public Changelog tab. Owns `changelog_entries`.

| Item | Contract |
| --- | --- |
| `ReleaseInput::validate()` | Checks version, day, title and the four boxes (Added, Changed, Fixed, Removed; one change per line, a leading `-` or `*` is dropped). Returns the sections, or `ChangelogError::Invalid` with every bad field at once. At least one change is needed. |
| `create`, `update`, `delete` | Team writes. `ChangelogError::Taken` for a version the project already has. The owner or a super admin only. |
| `page(db, project_id, q, sort, page)` | The team's list, 25 a page (`caps::CHANGELOG_PAGE`), searched by version or title, newest first unless `sort` is `oldest`. |
| `index(db, project_id, q, before)` | The public version index, newest first, searched on the server, keyset by `(released_on, id)`. |
| `one(db, project_id, version)` | One release with its newer and older neighbours. `None` for the version means the newest; the route spells that `latest`. |

The public routes answer 404 while `show_changelog` is off.

## 5.11 maintenance

**Responsibility:** planned work on a project (v1.1 decision Q18). Owns `maintenance_windows` and `maintenance_items`. Calls `checks::source_names`; `incidents` and `checks` call it.

| Item | Contract |
| --- | --- |
| `state(starts_at, ends_at, now)` | `Scheduled`, `Active` or `Ended`, from the times alone. |
| `create(db, project_id, &WindowInput, now)` | Checks the form on a read connection, then again inside the write. A past start becomes now. `MaintenanceError::Invalid` names every bad field; `Full` at 20 windows scheduled or active (`caps::OPEN_WINDOWS_PER_PROJECT`). |
| `delete(db, project_id, id, now)` | Scheduled windows only; otherwise `Started`. |
| `end_now(db, project_id, id, now)` | Active windows only: `ends_at` becomes now. Otherwise `NotActive`. |
| `page(db, project_id, page)` | The team's list, 20 a page (`caps::WINDOWS_PAGE`), latest start first. |
| `covers(c, kind, id, at)` | The end of the latest-ending active window over the source, on the caller's connection. `incidents::observe` and `checks::heartbeat::sweep_missed` ask it. |
| `active(c, project_id, at)` | The active windows with their items, for the item states and the status page. |
