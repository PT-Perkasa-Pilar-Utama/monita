# 4. Checks

Rules: [../technical-specs/13-check-scheduler.md](../technical-specs/13-check-scheduler.md). Fields: [../technical-specs/06-data-model.md sections 6.3.7 and 6.3.8](../technical-specs/06-data-model.md#637-monitors).

## 4.1 `GET, HEAD, POST /ping/{token}`

| Part | Contract |
| --- | --- |
| Purpose | A project job reports a successful run. |
| Access | Anyone with the token. |
| Input | Path `token`. Any body is read and dropped, up to 16 MiB within 10 s of admission. A body that fits gets its answer through `curl -f`; a longer or slower one may see a reset. This route never answers 413. A ping refused at `admit` (unknown token, 429, or an `admit` error) is answered without reading its body. Monita records an admitted ping even if its client hangs up during the request. A client that half-closes its connection right after the request, as `nc -N` does, is not heard. Shutdown waits up to 10 s for a ping being stored (spec 13.4). |
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
| Input | Path `token`. Optional body: the first 1 KB of text is kept as the cause, cut on a character boundary. Leading whitespace is skipped and does not count toward the 1 KB. It still counts toward the 16 MiB cap and the 10 s. A longer body is cut, never refused, so a failure is never refused for its length. The body is read only after the token and the limits pass. The ping is stored at 1 KB of text, the end of the body, 16 MiB, or 10 s. `received_at` is the time Monita starts the store. Monita then reads the rest and drops it. One 10 s deadline from admission bounds both reads. The store between them is not bounded by it. One 16 MiB cap covers both reads. A body within both limits gets its answer through `curl -f`. A ping refused at `admit` is answered without reading its body (4.1). Monita records an admitted ping even if its client hangs up during the request. A client that half-closes its connection right after the request, as `nc -N` does, is not heard. Shutdown waits up to 10 s for a ping being stored (spec 13.4). |
| Behavior | Rate limit as 4.1, in its own bucket `pingfail:{id}`, so a fail right after a success ping is never refused. Insert `pings(kind=fail)`. `observe(Heartbeat, FailPing{cause})` opens an incident as soon as the ping is stored (AC-14.01). `deadline_at` does not move. |
| Output | 200 `OK` |
| Errors | Same as 4.1 |

## 4.3 Frontend routes

The React pages under `/projects/{slug}/monitors` and `/projects/{slug}/heartbeats` call these routes (conventions 1.10). A project another tech lead owns is 404 on every route. Numbers in a create or update body are strings, so a blank or bad number comes back as that field's 422 error. Lists are capped (spec 08 section 8.2), so the page's search and its State and Severity selects filter in the browser.

| Method | Path | Access | Input | Behavior | AC |
| --- | --- | --- | --- | --- | --- |
| GET | `/api/ui/check-options` | session | | `intervals` (30, 60, 180, 300 s with labels), `periods` and `graces` for heartbeats, and the defaults: a monitor checks every 1m with a 10 s timeout, expects `2xx` and is a warning; a heartbeat runs every day with 30 minutes grace and is a warning. | AC-09.02 |
| GET | `/api/ui/projects/{slug}/monitors` | owner, super_admin | | `monitors`: each with its settings, `state` (`up`, `down`, `issue`, `unknown` before the first check), `last` check (time, ok, a summary such as `200 in 182ms`) and `day`, the last 24 hours as sparkline geometry, failed hours, uptime and mean response. `summary`: the worst state and one sentence. | AC-10.01 |
| POST | `/api/ui/projects/{slug}/monitors` | owner, super_admin | `name`, `url`, `interval_s`, `timeout_s` (stored as `timeout_ms`), `expected_status`, `keyword`, `skip_tls_verify`, `severity`, `is_public` | Validate; 422 on each bad field. A full project is 422 on `name`. `next_check_at = now + 5 + rand(0..interval_s - 5)`. 201 `{id}`. | AC-09.01, AC-11.01 |
| GET | `/api/ui/projects/{slug}/monitors/{id}` | owner, super_admin | `?range=hour\|day\|week\|month` (anything else is a day); `?before=<checked_at>.<id>`, the cursor from `older` | `monitor`; `stats`: up or down for how long, the last check, the mean response time, incidents in 30 days. `insight`: the response-time chart for the range as geometry in a 960 x 240 view box (hour, day and week from the raw checks, month from the daily rollup and today), and availability for today, 7, 30 and 365 days and all time, from automatic incidents. `checks`: the last 7 days, 25 per page; `older` is the next cursor or null. | AC-10.01 |
| PUT | `/api/ui/projects/{slug}/monitors/{id}` | owner, super_admin | same as create | Validate and update, 204. | |
| DELETE | `/api/ui/projects/{slug}/monitors/{id}` | owner, super_admin | | Delete with its results; an open incident on it closes. 204. | |
| GET | `/api/ui/projects/{slug}/heartbeats` | owner, super_admin | | `heartbeats`: each with its settings, `state`, `last_ping_at`, `deadline_at` and `due` (how much of the wait has passed, and a sentence). `summary` as for monitors. | |
| POST | `/api/ui/projects/{slug}/heartbeats` | owner, super_admin | `name`, `period_s`, `grace_s`, `severity`, `is_public` | Create the token. `deadline_at = now + period_s + grace_s`. 201 `{id}`; a full project is 422 on `name`. | AC-12.01, AC-12.02 |
| GET | `/api/ui/projects/{slug}/heartbeats/{id}` | owner, super_admin | `?range=` as the monitor page; `?before=<received_at>.<id>` | `heartbeat`, `ping_url`, the same `stats` shape, `insight` with pings per bucket, then `pings` of the last 7 days, 25 per page, and `older`. | AC-12.02 |
| PUT | `/api/ui/projects/{slug}/heartbeats/{id}` | owner, super_admin | same as create | Validate and update, 204. | |
| DELETE | `/api/ui/projects/{slug}/heartbeats/{id}` | owner, super_admin | | Delete with its pings; its URL answers 404 from then on. 204. | |

## 4.4 System page

The React page `/admin/settings` calls these routes. A tech lead gets 403 on each.

| Method | Path | Access | Input | Behavior | AC |
| --- | --- | --- | --- | --- | --- |
| GET | `/api/ui/admin/system` | super_admin | | `mail` (server and sender from config, never the password), `email` (counts for 7 days and the newest error), `server` (the configuration as label and value), `limits` (each runtime limit with its group, help, value, default, maximum and log-scale positions), and `caps` (the fixed caps, with the account count). | AC-11.03, AC-31.03 |
| POST | `/api/ui/admin/test-email` | super_admin | | Queue a test email to the signed-in admin through the outbox. `{sent_to}`. | |
| PUT | `/api/ui/admin/limits` | super_admin | each `settings` key, a number or a numeric string | Every key must be a whole number from 1 to its maximum, or nothing is saved and each bad key is 422 on that key. 204. | AC-11.03, AC-31.03 |

Validation: `interval_s` in {30, 60, 180, 300}. `timeout_s` 1 to 30. `expected_status` is a comma list of classes (`2xx`) or codes (`200`); spaces around commas are allowed, and `2xx,301` mixes both. `url` is `http` or `https`. A blank or non-numeric number is a 422 on that field.
