# 1. Conventions

## 1.1 Transport

| Item | Rule |
| --- | --- |
| Base | `{MONITA_BASE_URL}/api/v1` for JSON. `/ping` and `/healthz` sit at the root. |
| Content type | `application/json; charset=utf-8` for requests and responses |
| Body limit | 64 KB (SEC-19) |
| Time | Unix seconds (integer, UTC) in every JSON field ending in `_at` |
| IDs | Public tokens are strings. Internal numeric IDs never appear in the JSON API, except ticket `number`. |

## 1.2 Envelope

Success:

```json
{ "data": { } }
```

Collection:

```json
{ "data": [ ], "next_cursor": null }
```

Error:

```json
{ "error": { "code": "rate_limited", "message": "Too many requests. Retry in 12 s." } }
```

`/ping` returns plain text `OK` instead, so `curl` and cron logs stay readable.

## 1.3 Pagination

Cursor-based where a list can grow: `?cursor=<opaque>&limit=<1..100>`, default 50. `next_cursor` is `null` on the last page.

## 1.4 Status codes

| Status | Use |
| --- | --- |
| 200 | Read or action succeeded |
| 201 | Created |
| 204 | Done, no body |
| 400 | Malformed JSON or failed validation |
| 401 | Missing or bad credential |
| 403 | Credential valid, but not allowed (IP allowlist, Origin check) |
| 404 | Unknown resource or token |
| 409 | Conflict with current state |
| 422 | Valid JSON, but a business rule rejects it |
| 429 | Rate limit or daily cap |
| 503 | Monita unhealthy (`/healthz` only) |

## 1.5 Error codes

| code | status | condition |
| --- | --- | --- |
| `invalid_request` | 400 | Bad JSON or a field fails validation. `message` names the field. |
| `unauthorized` | 401 | No credential, or an unknown or revoked one |
| `forbidden` | 403 | IP outside the allowlist, or a failed Origin check |
| `not_found` | 404 | Unknown slug, token or resource |
| `ref_id_invalid` | 422 | Unknown, revoked or used-up ref ID |
| `rate_limited` | 429 | Over a rate limit (SEC-12) |
| `daily_cap_reached` | 429 | Over a ticket cap (SEC-13) |
| `internal` | 500 | Unexpected failure. Logged with a request ID. |

## 1.6 Rate-limit headers

Every 429 carries `Retry-After: <seconds>`. Limits are in [../technical-specs/07-security.md](../technical-specs/07-security.md) SEC-12 and SEC-13.

## 1.7 Request ID

Every response carries `X-Request-Id`. Logs use the same ID.

## 1.8 CORS

`/api/v1/public/*` and `/api/v1/tickets*` send `Access-Control-Allow-Origin: *`, so a project page can call them from the browser. `/api/v1/ref-ids` sends no CORS headers: only a backend may call it.

## 1.9 Roles

| Role | Credential | Matrix |
| --- | --- | --- |
| `super_admin`, `tl` | Session cookie | [../technical-specs/09-authentication-and-authorization.md section 9.3](../technical-specs/09-authentication-and-authorization.md#93-role-matrix) |
| project backend | `Authorization: Bearer mk_...` | same |
| job | heartbeat token in the path | same |
| reporter | ref ID | same |
| public | none | same |

## 1.10 HTML routes

HTML routes return a full page, or a fragment when the request carries `HX-Request: true`. Form posts use `application/x-www-form-urlencoded`. A validation failure re-renders the form with status 422. A success on a form post redirects with 303, or sends `HX-Redirect` for htmx.
