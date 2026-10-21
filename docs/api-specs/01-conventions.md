# 1. Conventions

## 1.1 Transport

| Item | Rule |
| --- | --- |
| Base | `{MONITA_BASE_URL}/api/v1` for JSON. `/ping` and `/healthz` sit at the root. |
| Content type | `application/json; charset=utf-8` for requests and responses |
| Body limit | Request bodies are capped at 128 KiB (413) (SEC-19). The ping routes never answer 413. They read an admitted body up to 16 MiB or 10 s and keep at most 1 KB |
| Time | Unix seconds (integer, UTC) in every JSON field ending in `_at` |
| IDs | Public tokens are strings. Internal numeric IDs never appear in the JSON API, except ticket `number`. |

## 1.2 Envelope

Success:

```json
{ "data": { } }
```

Collection:

```json
{ "data": [ ] }
```

Error:

```json
{ "error": { "code": "rate_limited", "message": "Too many requests. Retry in 12 s." } }
```

`/ping` returns plain text `OK` instead, so `curl` and cron logs stay readable.

## 1.3 Pagination

A list that can grow is capped or paged. Each endpoint states which.

| Kind | How | Where |
| --- | --- | --- |
| Offset page | `?page=` from 1. A missing, bad or lower value reads page 1. The answer carries `page`, `page_size` and `total`. | Projects (20, 3.1), cross-project Incidents and Tickets (25, 5.2, 8.6), team Changelog (25, 11.1), one ticket column (25, 8.5) |
| Keyset page | `?before=<time>.<id>`. The answer carries `older`, the next cursor or null. A bad cursor reads the first page. | Checks and pings (25, 4.3), public history (25, 7.2), public Changelog (25, 11.2), public incidents by month (7.2) |
| Cap | The newest rows up to a fixed number, no cursor. | The board (25 a column, 8.5), the reporter's tickets (100, 8.3), a project's incidents (200, 5.1), a project's monitors and heartbeats (100 each, 4.3) |

## 1.4 Status codes

| Status | Use |
| --- | --- |
| 200 | Read or action succeeded |
| 201 | Created |
| 204 | Done, no body |
| 400 | Malformed JSON or failed validation on `/api/v1/` |
| 401 | Missing or bad credential |
| 403 | Credential valid, but not allowed (IP allowlist, Origin check, a role without access, a supervisor's write) |
| 404 | Unknown resource or token |
| 410 | A link or token that worked once and no longer does (`/api/ui/`) |
| 409 | Conflict with current state |
| 413 | Request body over 128 KiB, except on the two ping routes (see Body limit) |
| 422 | A business rule rejects the request; on `/api/ui/`, also bad JSON and failed validation |
| 429 | Rate limit or daily cap |
| 503 | Monita unhealthy (`/healthz` only) |

## 1.5 Error codes

| code | status | condition |
| --- | --- | --- |
| `invalid_request` | 400 | `/api/v1/`: bad JSON or a field fails validation. `message` names the field. |
| `validation_failed` | 422 | `/api/ui/`: bad JSON or a refused form. `fields` maps each field to its message (1.10). |
| `gone` | 410 | `/api/ui/`: a used or expired invite |
| `too_large` | 413 | A body over 128 KiB |
| `unauthorized` | 401 | No credential, or an unknown or revoked one |
| `forbidden` | 403 | IP outside the allowlist, a failed Origin check, a role without access, or any write by a supervisor (1.9) |
| `not_found` | 404 | Unknown slug, token or resource |
| `ref_id_invalid` | 422 | Unknown, revoked or used-up ref ID |
| `rate_limited` | 429 | Over a rate limit (SEC-12) |
| `daily_cap_reached` | 429 | Over a ticket cap (SEC-13) |
| `internal` | 500 | Unexpected failure. The cause is logged. The body says only "Unexpected failure." on `/api/v1/`, and "The error is in the server log. Try again in a moment." on `/api/ui/`. |

## 1.6 Rate-limit headers

Every 429 carries `Retry-After: <seconds>`. Limits are in [../technical-specs/07-security.md](../technical-specs/07-security.md) SEC-12 and SEC-13.

## 1.7 Request ID

v1 sends no request ID. A 500 is matched to its log line by time and path.

## 1.8 CORS

`/api/v1/public/*` and `/api/v1/tickets*` send `Access-Control-Allow-Origin: *`, so a project page can call them from the browser. `/api/v1/ref-ids` sends no CORS headers: only a backend may call it.

## 1.9 Roles

| Role | Credential | Matrix |
| --- | --- | --- |
| `super_admin`, `tl`, `supervisor` | Session cookie | [../technical-specs/09-authentication-and-authorization.md section 9.3](../technical-specs/09-authentication-and-authorization.md#93-role-matrix) |
| project backend | `Authorization: Bearer mk_...` | same |
| job | heartbeat token in the path | same |
| reporter | ref ID | same |
| public | none | same |

A `super_admin` changes anything. A `tl` manages only the projects they own; a project of another owner is 404. A `supervisor` sees every project and the admin pages and changes nothing.

### The supervisor rule

Every signed-in route takes the `CurrentUser` extractor. For a supervisor it answers 403 `forbidden` to any method but GET and HEAD, before the handler runs. So a write by a supervisor is 403 on every route, whatever its body. A page request (not under `/api/`) gets the 403 page instead.

A GET shows a supervisor the same JSON as anyone else, with these values replaced. A value that is not set stays `null` or empty.

| Route | Field | A supervisor gets |
| --- | --- | --- |
| `GET /api/ui/projects/{slug}/heartbeats/{id}` | `ping_url` | `null`. With the token a supervisor could keep a dead job alive. |
| `GET /api/ui/projects/{slug}/monitors` and `.../monitors/{id}` | `headers` | Each line as `Name: (hidden)`. A line with no colon reads `(hidden)`. |
| same | `body` | `"(hidden)"` when a body is set |
| same | `auth_user` | `"(hidden)"` when set |
| `GET /api/ui/projects/{slug}/alert-cards` | `url` and `key` of each card, ticket cards too | `"(hidden)"` when set |

`GET /api/ui/me` tells the page which role it has (2.2). The web app hides its create, edit and delete controls when `read_only` is true.

## 1.10 The frontend and its API

The pages are a React app in `web/`, embedded in the binary. A GET to any path no route claims returns its `index.html` with `Cache-Control: no-cache`; its hashed files live under `/assets/` with a one-year cache. A path under `/api/`, `/ping/`, `/static/` or `/assets/` that no route claims is a 404, in the envelope under `/api/`. A write to a path no route claims is also a 404, so an unregistered route such as reset-state in prod looks absent (SEC-17).

The app reads and writes through `/api/ui/...`:

- JSON bodies, the envelope in 1.2, and the session cookie for identity. No session is 401 `unauthorized`; a role without access is 403 `forbidden`.
- Every write carries `Origin: MONITA_BASE_URL`, which a browser's `fetch` sends (SEC-03). Without it the answer is 403.
- A refused form is 422 `validation_failed`. `error.message` is the first problem; `error.fields` maps a form field to its message, so the page marks that field.
- A dead link or token is 410 `gone`.
- A body over 128 KiB is 413 `too_large`.
- A write that reveals a one-time secret (a new API key, an invite link) returns it in the 2xx body once. It is never in a URL.
- Every `/api/ui/` response outside `/api/ui/public/` carries `Cache-Control: no-store`, so a one-time secret never sits in a cache.
- A public read (status, history, uptime, incidents by month, the public ticket board, the public Changelog) carries `Cache-Control: public, max-age=30`; a refused one carries no cache header.
- A daily ticket cap is 429 `daily_cap_reached`.
