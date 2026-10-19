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

The React pages under `/projects/{slug}/monitors` and `/projects/{slug}/heartbeats` call these routes (conventions 1.10). A project another tech lead owns is 404 on every route. A supervisor may read and gets 403 on every write; the read hides some values (conventions 1.9). In a create or update body, numbers that a form types are strings, so a blank or bad number comes back as that field's 422 error. `auth_secret_id` is a number or null. Switches (`skip_tls_verify`, `is_public`, `alert`, `auto_incident`) are booleans. Lists are capped (100 a project), so the page's search and its State and Severity selects filter in the browser. Every refused form is 422 with all its bad fields in `error.fields`.

| Method | Path | Access | Input | Behavior | AC |
| --- | --- | --- | --- | --- | --- |
| GET | `/api/ui/check-options` | session | | `intervals` (30, 60, 180, 300 s, each with its label in words), `periods` and `graces` for heartbeats (value in seconds and label), `fails` (`{min: 1, max: 99}`, the failure count's range) and the defaults. `monitor`: every 60 s, `timeout_s` 10, expects `2xx`, a warning. `heartbeat`: every day (`period_s` 86400), 30 minutes late (`grace_s` 1800), a warning. | AC-09.02 |
| GET | `/api/ui/projects/{slug}/monitors` | owner, super_admin, supervisor | | `monitors`: each a monitor (below) with `day`, the last 24 hours as sparkline geometry (`path`, `fails`, `uptime`, `mean_ms`; null before the first check). `summary`: `{kind, text}`, the worst state and one sentence. | AC-10.01 |
| POST | `/api/ui/projects/{slug}/monitors` | owner, super_admin | the monitor form (below) | Validate; 422 on each bad field. A project with 100 monitors is 422 on `name`. A `auth_secret_id` that is not a secret of this project is 422 on `auth_secret_id`. `next_check_at = now + 5 + rand(0..interval_s - 5)`. 201 `{id}`. | AC-09.01, AC-11.01 |
| GET | `/api/ui/projects/{slug}/monitors/{id}` | owner, super_admin, supervisor | `?range=hour\|day\|week\|month` (anything else is a day); `?before=<checked_at>.<id>`, the cursor from `older` | `monitor` (also `auth_secret_name`, the name of the secret it signs in with, never its value); `stats`: four cards `{label, value, sub, tone}` (up or down for how long, the last check, the mean response time, incidents in 30 days). `insight`: the response-time chart for the range as geometry in a 960 x 240 view box (hour, day and week from the raw checks, month from the daily rollup and today), `total`, `ok`, `mean_ms`, and `periods`, the availability for today, 7, 30 and 365 days and all time (`availability`, `downtime_s`, `incidents`, `longest_s`, `average_s`), from automatic incidents. `checks`: the last 7 days, 25 per page, each `{id, checked_at, ok, status_code, latency_ms, error}`; `older` is the next cursor or null. | AC-10.01 |
| PUT | `/api/ui/projects/{slug}/monitors/{id}` | owner, super_admin | same as create | Validate and update, 204. An unknown id is 404. A changed interval draws a new first-check offset. | |
| DELETE | `/api/ui/projects/{slug}/monitors/{id}` | owner, super_admin | | Delete with its results; an open incident on it closes. 204. | |
| GET | `/api/ui/projects/{slug}/heartbeats` | owner, super_admin, supervisor | | `heartbeats`: each a heartbeat (below). `summary` as for monitors. | |
| POST | `/api/ui/projects/{slug}/heartbeats` | owner, super_admin | `name`, `period_s`, `grace_s`, `severity`, `is_public`, `alert`, `auto_incident` | Create the token. `deadline_at = now + period_s + grace_s`. 201 `{id}`; a project with 100 heartbeats is 422 on `name`. | AC-12.01, AC-12.02 |
| GET | `/api/ui/projects/{slug}/heartbeats/{id}` | owner, super_admin, supervisor | `?range=` as the monitor page; `?before=<received_at>.<id>` | `heartbeat`; `ping_url`, the address a job calls (`null` for a supervisor); the same `stats` shape with four cards (up or down for how long, next ping due, last ping, incidents); `insight` with pings per bucket; then `pings` of the last 7 days, 25 per page, each `{id, received_at, kind, source_ip, cause}`, and `older`. `cause` is what a fail ping reported, the first 1 KB. A fail ping counts as heard. | AC-12.02 |
| PUT | `/api/ui/projects/{slug}/heartbeats/{id}` | owner, super_admin | same as create | Validate and update, 204. A pending deadline is recomputed from the last ping with the new period and grace. | |
| DELETE | `/api/ui/projects/{slug}/heartbeats/{id}` | owner, super_admin | | Delete with its pings; its URL answers 404 from then on. An open incident on it closes. 204. | |

### Monitor

The monitor form sends these fields. The view returns the same fields, plus those marked view.

| Field | Type | Rule |
| --- | --- | --- |
| `name` | string | 1 to 80 characters |
| `url` | string | starts with `http://` or `https://` |
| `method` | string | `GET`, `HEAD`, `POST` or `PUT`; blank is `GET` |
| `headers` | string | `Name: value` lines. Up to 20 headers and 4 KB a value. `Host` and `Content-Length` are refused. A header may not repeat the one that Authentication sends. |
| `body` | string | Sent with `POST` and `PUT` only, up to 64 KB. A body with another method is 422 on `body`. Blank is none. |
| `interval_s` | string | 30, 60, 180 or 300 |
| `timeout_s` | string | 1 to 30 (stored as `timeout_ms`) |
| `expected_status` | string | A comma list of classes (`2xx`) or codes (`200`); spaces around commas are allowed, and `2xx,301` mixes both. Each part is three characters, first digit 1 to 5. The error reads "Use a class such as 2xx, or a list of codes such as 200,204." |
| `keyword` | string | Optional text the response must contain, up to 200 characters |
| `skip_tls_verify` | boolean | Accept a self-signed certificate |
| `severity` | string | `critical` or `warning` |
| `is_public` | boolean | Show it on the status page |
| `fails_to_open` | string | Failed checks in a row that count as down, 1 to 99. Missing means 2. |
| `alert` | boolean | A failure alerts the severity's recipients. Missing means on. Off sends nothing. |
| `auto_incident` | boolean | A failure opens an incident by itself. Missing means on. Off: no incident opens, a failure sends the alert `source.down` and a recovery `source.up` (6.2), the team's lists show Down, and the status page shows nothing. |
| `auth_kind` | string | `none`, `bearer`, `basic` or `header`; blank is `none` |
| `auth_secret_id` | number or null | The project secret (3.1) that the check sends. Required unless `auth_kind` is `none`. Another project's secret is 422. |
| `auth_user` | string | The user name for `basic`: up to 200 characters, no colon |
| `auth_header` | string | The header name for `header`, such as `X-Api-Key` |

View fields: `id`, `every` (the interval in words: `30 seconds`, `1 minute`, `3 minutes`, `5 minutes`), `timeout_s`, `created_at`, `next_check_at`, `state` (`up`, `down`, `issue`, or `unknown` before the first check) and `last` (`{checked_at, ok, summary}`, a summary such as `200 in 182ms`, or null). `bearer` sends `Authorization: Bearer <secret>`; `basic` sends the user name and the secret as the password; `header` sends the secret in the named header. Header values, the body and the secret never reach a check error, a log or the status page. When a monitor's secret is deleted, its check fails with a fixed sentence, and the monitor's form asks for a new one. A secret in use cannot be deleted (3.1).

### Heartbeat

The form sends `name` (1 to 80 characters), `period_s`, `grace_s` (both from the lists in `check-options`), `severity`, `is_public`, `alert` and `auto_incident` (as for a monitor). The page words `grace_s` as "Can be late by"; the field name and the 422 key stay `grace_s`, and its error reads "Pick how late the job can be from the list."

View fields: `id`, `every` and `grace` (in words, such as `1 day`, `30 minutes`), `created_at`, `last_ping_at`, `deadline_at`, `state`, `due` (`{used, late, text}`: how much of the wait has passed, and a sentence) and `last` (`{ok, cause, at}`, the newest ping of either kind, or null).

## 4.4 System page

The React pages under `/admin` (`system`, `limits`, `email`, `time-zone`) call these routes. A tech lead gets 403 on each. A supervisor may read the System view and gets 403 on every write.

| Method | Path | Access | Input | Behavior | AC |
| --- | --- | --- | --- | --- | --- |
| GET | `/api/ui/admin/system` | super_admin, supervisor | | `mail` (the mail server in use, see below; never the password), `email` (`sent`, `failed`, `waiting`, `last_sent_at` and `last_error` for the last `email_days` = 7 days), `server` (the configuration as label and value), `limits` (each runtime limit with its `key`, `label`, `group`, `help`, `value`, `default`, `max`, and the log-scale positions `at` and `default_at`), `groups`, and `caps` (the fixed caps, with the account count), and `utc_offset_min`. | AC-11.03, AC-31.03 |
| PUT | `/api/ui/admin/mail` | super_admin | `host`, `port`, `security` (`tls`, `starttls` or `none`), `user`, `password`, `remove_password`, `from`, `skip_verify` | Save the mail server. A blank `password` keeps the saved one; `remove_password` clears it. A password needs a `user`. `from` is an address such as `Monita <monita@company.co.id>`. 204, or 422 with every bad field in `error.fields`. The password is stored as entered, because it is sent to the server (technical-specs 07). | |
| DELETE | `/api/ui/admin/mail` | super_admin | | Remove the saved mail server; `MONITA_SMTP_URL` or the log applies. 204. | |
| POST | `/api/ui/admin/mail/test-connection` | super_admin | the same fields as the save | Try the form's values without saving: connect, TLS, sign in. A blank password uses the saved one. `{ok, server}` or `{ok: false, error}`, always 200; a refused field is 422. | |
| POST | `/api/ui/admin/mail/test-send` | super_admin | `to` | Send one email now through the mail server in use. `{ok, to, source, error?}`, always 200; a bad address is 422 on `to`. | |
| PUT | `/api/ui/admin/limits` | super_admin | each `settings` key, a number or a numeric string | Every key must be a whole number from 1 to its maximum, or nothing is saved and each bad key is 422 on that key. The keys: `check_concurrency` (max 500), `rl_ip_per_min`, `rl_key_per_min`, `rl_ping_per_s`, `tickets_per_ref_per_day`, `tickets_per_project_per_day` (max 1 000 000 each). 204. | AC-11.03, AC-31.03 |
| PUT | `/api/ui/admin/time-zone` | super_admin | `utc_offset_min` | Set the offset that every page shows times in: minutes east of UTC, -720 to 840, in steps of 15. Anything else is 422 on `utc_offset_min`. Default 0. 204. | |

`mail` in the System view: `source` (`saved` for this page, `env` for `MONITA_SMTP_URL`, `none` for the log), `server` (`scheme://host:port`, or null), `user`, `from`, `host`, `port`, `security`, `skip_verify` (the last three are null unless the source is `saved`), and `has_password`.

The offset is a fixed number of minutes, with no daylight saving. It sets chart labels, "since" lines, availability for Today, the status page's day bars and the pulse hours. Daily summaries and the month chart stay on UTC days. Ticket daily caps, retention and the digest hour stay on UTC.

