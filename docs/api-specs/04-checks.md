# 4. Checks

Rules: [../technical-specs/13-check-scheduler.md](../technical-specs/13-check-scheduler.md). Fields: [../technical-specs/06-data-model.md sections 6.3.7 and 6.3.8](../technical-specs/06-data-model.md#637-monitors).

## 4.1 `GET, HEAD, POST /ping/{token}`

| Part | Contract |
| --- | --- |
| Purpose | A project job reports a successful run. |
| Access | Anyone with the token. |
| Input | Path `token`. Any body is ignored. |
| Behavior | 1. Find the heartbeat by token. 2. Rate limit: 1 per second per heartbeat, in the bucket `ping:{id}`. Over the limit: 429. A refused ping is not stored; the ping accepted in that second keeps the heartbeat alive (AC-13.02). 3. Insert `pings(kind=ok)`. 4. Set `last_ping_at = now`, `deadline_at = now + period_s + grace_s`. 5. `incidents::observe(Heartbeat, Ping)`, which closes an open incident (AC-18.01). |
| Output | 200 `text/plain` `OK`. HEAD has no body. (AC-13.01) |
| Errors | 404 `not_found` (plain text `not found`) for an unknown token. 429 with `Retry-After: 1` when over the limit. More than 60 unknown-token pings per minute from one IP (IPv6: one /64) get 429 before any lookup. |

Example:

```bash
curl -fsS -m 10 https://monita.company.co.id/ping/k3j9w2x7q4m8p6r1t5v0y2z8ab
```

## 4.2 `GET, HEAD, POST /ping/{token}/fail`

| Part | Contract |
| --- | --- |
| Purpose | A project job reports its own failure. |
| Access | Anyone with the token. |
| Input | Path `token`. Optional body: the first 1 KB of text is kept as the cause, cut on a character boundary. A longer body is cut, never refused, so the failure is always recorded. Only a body over 128 KiB is 413, as on every route (SEC-19). |
| Behavior | Rate limit as 4.1, in its own bucket `pingfail:{id}`, so a fail right after a success ping is never refused. Insert `pings(kind=fail)`. `observe(Heartbeat, FailPing{cause})` opens an incident at once (AC-14.01). `deadline_at` does not move. |
| Output | 200 `OK` |
| Errors | Same as 4.1 |

## 4.3 HTML routes

| Method | Path | Access | Input | Behavior | AC |
| --- | --- | --- | --- | --- | --- |
| GET | `/projects/{slug}/monitors` | owner, super_admin | | List with current state and last check | |
| GET | `/projects/{slug}/monitors/new` | owner, super_admin | | Form with defaults: 1m, 10 s, `2xx`, warning, hidden | AC-09.02 |
| POST | `/projects/{slug}/monitors` | owner, super_admin | `name`, `url`, `interval_s`, `timeout_s` (stored as `timeout_ms`), `expected_status`, `keyword`, `skip_tls_verify`, `severity`, `is_public` | Validate. `next_check_at = now + 5 + rand(0..interval_s - 5)`. | AC-09.01, AC-11.01 |
| GET | `/projects/{slug}/monitors/{id}` | owner, super_admin | `?before=<checked_at>.<id>`, the cursor the page links to | Detail and checks of the last 7 days: time, status, latency, error | AC-10.01 |
| POST | `/projects/{slug}/monitors/{id}` | owner, super_admin | same as create | Update | |
| POST | `/projects/{slug}/monitors/{id}/delete` | owner, super_admin | | Delete with results | |
| GET | `/projects/{slug}/heartbeats` | owner, super_admin | | List with state, last ping, ping URL | |
| POST | `/projects/{slug}/heartbeats` | owner, super_admin | `name`, `period_s`, `grace_s`, `severity`, `is_public` | Create token. `deadline_at = now + period_s + grace_s`. | AC-12.01, AC-12.02 |
| GET | `/projects/{slug}/heartbeats/{id}` | owner, super_admin | | Detail, ping URL, recent pings | AC-12.02 |
| POST | `/projects/{slug}/heartbeats/{id}` | owner, super_admin | same as create | Update | |
| POST | `/projects/{slug}/heartbeats/{id}/delete` | owner, super_admin | | Delete | |
| GET | `/admin/settings` | super_admin | | Limits and the check cap | AC-11.03, AC-31.03 |
| POST | `/admin/settings` | super_admin | each `settings` key | Validate positive integers. Save. | AC-11.03, AC-31.03 |

Validation: `interval_s` in {30, 60, 180, 300}. `timeout_s` 1 to 30. `expected_status` is a comma list of classes (`2xx`) or codes (`200`); spaces around commas are allowed, and `2xx,301` mixes both. `url` is `http` or `https`. A blank or non-numeric number re-renders the form with the field's error.
