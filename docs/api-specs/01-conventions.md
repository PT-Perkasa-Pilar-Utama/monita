# 1. Conventions

## 1.1 Transport

| Item | Rule |
| --- | --- |
| Base | `{MONITA_BASE_URL}/api/v1` for JSON. `/ping` and `/healthz` sit at the root. |
| Content type | `application/json; charset=utf-8` for requests and responses |
| Body limit | Request bodies are capped at 128 KiB (413) (SEC-19). The ping routes never answer 413: `/ping/{token}` ignores its body; `/ping/{token}/fail` keeps 1 KB, then reads the rest up to 16 MiB or 10 s after its token passes |
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

JSON lists are capped, not paged. Each endpoint states its cap, for example the newest 100 tickets in 8.3.

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
| 413 | Request body over 128 KiB, except on the two ping routes (see Body limit) |
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
| `internal` | 500 | Unexpected failure. The cause is logged; the body says only "Unexpected failure." |

## 1.6 Rate-limit headers

Every 429 carries `Retry-After: <seconds>`. Limits are in [../technical-specs/07-security.md](../technical-specs/07-security.md) SEC-12 and SEC-13.

## 1.7 Request ID

v1 sends no request ID. A 500 is matched to its log line by time and path.

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

HTML routes return a full page. Form posts use `application/x-www-form-urlencoded`. A validation failure re-renders the form with status 422. A success on a form post redirects with 303. Exception: a post that reveals a one-time secret (a new API key, an invite link) renders it with 200, because the secret must not travel in a URL; a reload creates another key or invite, never a duplicate of the same one.
