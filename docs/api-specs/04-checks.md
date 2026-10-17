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

## 4.3 HTML routes

| Method | Path | Access | Input | Behavior | AC |
| --- | --- | --- | --- | --- | --- |
| GET | `/projects/{slug}/monitors` | owner, super_admin | | List with current state and last check | |
| GET | `/projects/{slug}/monitors/new` | owner, super_admin | | Form with defaults: 1m, 10 s, `2xx`, warning, hidden | AC-09.02 |
| POST | `/projects/{slug}/monitors` | owner, super_admin | `name`, `url`, `interval_s`, `timeout_s` (stored as `timeout_ms`), `expected_status`, `keyword`, `skip_tls_verify`, `severity`, `is_public` | Validate. `next_check_at = now + 5 + rand(0..interval_s - 5)`. | AC-09.01, AC-11.01 |
| GET | `/projects/{slug}/monitors/{id}` | owner, super_admin | `?range=hour\|day\|week\|month` (anything else is a day); `?before=<checked_at>.<id>`, the cursor the page links to | Up or down for how long, the last check, the mean response time, incidents in 30 days. A response-time chart for the range: hour, day and week from the raw checks, month from the daily rollup and today. Availability for today, 7, 30 and 365 days and all time, from automatic incidents. Checks of the last 7 days, 25 per page. | AC-10.01 |
| POST | `/projects/{slug}/monitors/{id}` | owner, super_admin | same as create | Update | |
| POST | `/projects/{slug}/monitors/{id}/delete` | owner, super_admin | | Delete with results | |
| GET | `/projects/{slug}/heartbeats` | owner, super_admin | | List with state, last ping, ping URL | |
| POST | `/projects/{slug}/heartbeats` | owner, super_admin | `name`, `period_s`, `grace_s`, `severity`, `is_public` | Create token. `deadline_at = now + period_s + grace_s`. | AC-12.01, AC-12.02 |
| GET | `/projects/{slug}/heartbeats/{id}` | owner, super_admin | `?range=` as the monitor page; `?before=<received_at>.<id>`, the cursor the page links to | Detail, ping URL, the same stats, chart and availability as the monitor page with pings per bucket, then pings of the last 7 days, 25 per page | AC-12.02 |
| POST | `/projects/{slug}/heartbeats/{id}` | owner, super_admin | same as create | Update | |
| POST | `/projects/{slug}/heartbeats/{id}/delete` | owner, super_admin | | Delete | |
| GET | `/admin/settings` | super_admin | `?sent=1` after a test email | The System page: email delivery (server and sender from config, never the password; email counts for 7 days; the newest error), the server's configuration, the limits, and the fixed caps with the project and account counts | AC-11.03, AC-31.03 |
| POST | `/admin/settings/test-email` | super_admin | | Queue a test email to the signed-in admin through the outbox. Redirect to `?sent=1#email`. | |
| POST | `/admin/settings` | super_admin | each `settings` key | Validate positive integers. Save. | AC-11.03, AC-31.03 |

Validation: `interval_s` in {30, 60, 180, 300}. `timeout_s` 1 to 30. `expected_status` is a comma list of classes (`2xx`) or codes (`200`); spaces around commas are allowed, and `2xx,301` mixes both. `url` is `http` or `https`. A blank or non-numeric number re-renders the form with the field's error.
