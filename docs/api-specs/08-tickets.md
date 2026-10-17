# 8. Tickets

Fields and stage rules: [../technical-specs/06-data-model.md sections 6.3.13 and 6.4.2](../technical-specs/06-data-model.md#6313-ref_ids-and-tickets).

## 8.1 `POST /api/v1/ref-ids`

| Part | Contract |
| --- | --- |
| Purpose | A project backend gets a ref ID for one of its users or for one ticket. |
| Access | project API key ([03 section 3.2](03-projects.md#32-api-key-rule-for-every-json-call)). The project comes from the key. |
| Input | Body below |
| Behavior | Generate 128 random bits, base32. Store the hash with `single_use`. Return the raw ID once. (AC-23.01, AC-23.02) |
| Errors | 401 `unauthorized` (AC-23.03), 403 `forbidden`, 400 `invalid_request`, 429 `rate_limited` |

| field | type | required | notes |
| --- | --- | --- | --- |
| `single_use` | boolean | no | Default `false` |

Output 201:

```json
{ "data": { "ref_id": "rf_5k2m9q7w3x8p1t6v4y0z2a8bcd", "single_use": false, "created_at": 1792137600 } }
```

## 8.2 `POST /api/v1/tickets`

| Part | Contract |
| --- | --- |
| Purpose | Submit a bug ticket. Called by the project UI or backend. |
| Access | Anyone with a valid ref ID. 60/min per IP. CORS `*`. |
| Behavior | 1. Validate fields. 2. Find the ref ID by hash. Unknown, revoked or used up: 422 (AC-24.03, AC-27.01). 3. Count today's tickets (UTC) for the ref ID and the project. Over 5 or 100: 429 `daily_cap_reached` (AC-24.04, AC-24.05). 4. In one transaction: insert the ticket with `stage = triage` and the next `number`, and mark a single-use ref ID used (AC-25.02). 5. `notify::send(TicketCreated)` (AC-30.03). |
| Errors | 400, 422 `ref_id_invalid`, 429 `daily_cap_reached`, 429 `rate_limited` |

| field | type | required | notes |
| --- | --- | --- | --- |
| `ref_id` | string | yes | |
| `title` | string | yes | 3 to 120 chars |
| `description` | string | yes | 1 to 10 000 chars |
| `page_url` | string | no | `http` or `https`, up to 2 000 chars |

Output 201:

```json
{ "data": { "number": 17, "stage": "triage", "created_at": 1792137600 } }
```

(AC-24.02)

## 8.3 `GET /api/v1/tickets/mine`

| Part | Contract |
| --- | --- |
| Purpose | A reporter lists their own tickets. |
| Access | Header `X-Ref-Id: <ref_id>`. Header, not query, so the ID stays out of access logs. 60/min per IP. CORS `*`. |
| Behavior | Find the ref ID, including revoked or used ones. Return its newest 100 tickets, newest first, including Triage. (AC-29.01) |
| Errors | 401 `unauthorized` for a missing header. 422 `ref_id_invalid` for an unknown ID. |

Output 200:

```json
{
  "data": [
    { "number": 17, "title": "Checkout button does nothing", "stage": "in_progress",
      "close_reason": null, "created_at": 1792137600, "updated_at": 1792224000 }
  ]
}
```

The reporter sees their original title. The description is not returned.

## 8.4 CORS preflight

`OPTIONS /api/v1/tickets` and `OPTIONS /api/v1/tickets/mine` answer 204 with `Access-Control-Allow-Origin: *`, `Allow-Methods: GET, POST, OPTIONS`, `Allow-Headers: content-type, x-ref-id` and `Max-Age: 86400`. Preflights do not count against the per-IP limit.

## 8.5 Frontend routes

The React pages `/status/{slug}/report`, `/status/{slug}/mine` and `/status/{slug}/tickets` call the public routes below with no sign-in; the per-IP limit covers `/api/ui/public/` (SEC-12). The pages `/projects/{slug}/tickets` and `/projects/{slug}/tickets/{number}` call the tech lead's routes (conventions 1.10). A refused form is 422 with the error on its field: `ref_id`, `title`, `description`, `page_url`, `stage`, `close_reason` or `public_title`. SEC-14: no public route returns a description, a reporter, a close reason or a Triage ticket.

| Method | Path | Access | Input | Behavior | AC |
| --- | --- | --- | --- | --- | --- |
| GET | `/api/ui/public/{slug}` | public | | The project's `name` and `slug`, for a public page's header. Unknown slug: 404. | |
| POST | `/api/ui/public/{slug}/report` | public, 60/min per IP | `ref_id`, `title`, `description`, `page_url` | Same rules as 8.2. The ref ID must belong to this project. 201 `{number}`; a cap is 429 `daily_cap_reached`. | AC-24.01, AC-24.03 to AC-24.05 |
| POST | `/api/ui/public/{slug}/mine` | public, 60/min per IP | `ref_id` | `tickets`: the same data as 8.3. The ref ID must belong to this project. A POST keeps the ref ID out of URLs and logs. | AC-29.01 |
| GET | `/api/ui/public/{slug}/tickets` | public | | Public board: stages Backlog, In progress, Done. `tickets` with number, public title or title, stage, created, updated; `counts` per stage. Each column holds its newest 200; the count is every ticket. `Cache-Control: public, max-age=30`. | AC-28.01, AC-28.02, AC-26.01 |
| GET | `/api/ui/projects/{slug}/tickets` | owner, super_admin | | Ticket board: Triage, Backlog, In progress, Done. `tickets` (newest 200 per column) and `counts` per stage. | AC-25.01 |
| GET | `/api/ui/projects/{slug}/tickets/{number}` | owner, super_admin | | Full ticket, including description and whether its ref ID is active or revoked. Only the ref ID's hash is stored, so no part of it can be shown. | |
| PUT | `/api/ui/projects/{slug}/tickets/{number}/stage` | owner, super_admin | `stage`, `close_reason` (required for `done`) | Move, 204. Missing reason for `done`: 422 on `close_reason`. Leaving `done` clears the reason. | AC-25.03 |
| PUT | `/api/ui/projects/{slug}/tickets/{number}/public-title` | owner, super_admin | `public_title` (empty clears it) | Save, 204. | AC-26.01 |
| POST | `/api/ui/projects/{slug}/tickets/{number}/revoke-ref` | owner, super_admin | | Revoke the ticket's ref ID, 204. Tickets stay. | AC-27.01, AC-27.02 |
