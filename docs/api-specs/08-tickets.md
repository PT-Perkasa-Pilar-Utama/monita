# 8. Tickets

Fields and stage rules: [../technical-specs/06-data-model.md sections 6.3.13 and 6.4.2](../technical-specs/06-data-model.md#6313-ref_ids-and-tickets).

## 8.1 `POST /api/v1/ref-ids`

| Part | Contract |
| --- | --- |
| Purpose | A project backend gets a ref ID for one of its users or for one ticket. |
| Access | project API key ([03 section 3.2](03-projects.md#32-api-key-rule-for-every-json-call)). The project comes from the key. |
| Input | Body below |
| Behavior | Generate 128 random bits, base32, with the prefix `rf_`. Store the hash with `single_use`, and the last 4 characters (`ref_tail`) so the team can tell IDs apart. Return the raw ID once. The same ID is available to the team from the board (8.5). (AC-23.01, AC-23.02) |
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
| Behavior | 1. Validate fields. 2. Find the ref ID by hash. Unknown, revoked or used up: 422 (AC-24.03, AC-27.01). 3. Count today's tickets (UTC) for the ref ID and the project. Over the per-ref and per-project limits (defaults 5 and 100; a super admin changes them in 4.4): 429 `daily_cap_reached` (AC-24.04, AC-24.05). 4. In one transaction: insert the ticket with `stage = triage` and the next `number`, and mark a single-use ref ID used (AC-25.02). 5. `notify::send(TicketCreated)` (AC-30.03). |
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

The public pages `/status/{slug}/issues` (known issues) and `/status/{slug}/ticket` (Report a bug and My tickets, one page) call the public routes below with no sign-in; the old paths `/tickets`, `/report` and `/mine` redirect there. The per-IP limit covers `/api/ui/public/` (SEC-12). The pages `/projects/{slug}/tickets`, `/projects/{slug}/tickets/done` and `/projects/{slug}/tickets/{number}` call the tech lead's routes (conventions 1.10). A supervisor may read and gets 403 on every write (conventions 1.9). A refused form is 422 with the error on its field: `ref_id`, `title`, `description`, `page_url`, `stage`, `close_reason` or `public_title`. SEC-14: no public route returns a description or a reporter. Without a ref ID, no public route returns a close reason or a Triage ticket; `mine` returns both, but only for the tickets that ref ID reported (8.3).

The project's `report` and `known_issues` flags (3.1, `public-pages`) gate the public routes: with `report` off, `report` and `mine` are 404; with `known_issues` off, the public board is 404.

A ticket on the team board and detail (`TicketRow`):

| Field | Value |
| --- | --- |
| `number`, `title`, `description`, `page_url` | as the reporter sent them |
| `public_title` | the title the public board shows instead, or null |
| `stage` | `triage`, `backlog`, `in_progress` or `done` |
| `close_reason` | `fixed`, `wont_fix` or `duplicate` when `stage` is `done`, else null |
| `created_at`, `updated_at` | Unix seconds |
| `filed_by` | the email of the team member who filed it; null for a reporter's ticket |
| `ref_tail` | the last 4 characters of the reporter's ref ID; null for a team ticket and for a ref ID issued before the tail was stored |
| `ref_issued_at` | when that ref ID was issued; null for a team ticket |
| `ref_revoked` | true when the ref ID is revoked. Only its hash is stored, so no more of it can be shown. |

| Method | Path | Access | Input | Behavior | AC |
| --- | --- | --- | --- | --- | --- |
| GET | `/api/ui/public/{slug}` | public | | The project's `name`, `slug`, `pages` (the public flags of 3.1), `utc_offset_min` and `homepage_url`, for a public page's header and its tabs. Unknown slug: 404. | |
| POST | `/api/ui/public/{slug}/report` | public, 60/min per IP | `ref_id`, `title`, `description`, `page_url` | Same rules as 8.2. The ref ID must belong to this project. 201 `{number}`; a cap is 429 `daily_cap_reached`. 404 with `report` off. | AC-24.01, AC-24.03 to AC-24.05 |
| POST | `/api/ui/public/{slug}/mine` | public, 60/min per IP | `ref_id` | `tickets`: the same data as 8.3. The ref ID must belong to this project, or it is 422 on `ref_id`. A POST keeps the ref ID out of URLs and logs. The page asks for the ref ID once, then shows a New report form beside the reporter's tickets. 404 with `report` off. | AC-29.01 |
| GET | `/api/ui/public/{slug}/tickets` | public | `?stage=` (`backlog`, `in_progress` or `done`) and `?page=` | Public board: stages Backlog, In progress, Done. `tickets` with number, public title or title, stage, created, updated; `counts` per stage. Each column holds its newest 25 by last change. With `?stage=`, one page of that column: `{tickets, page, total, page_size}`; the Triage stage is always empty and an unknown stage is 404. `Cache-Control: public, max-age=30`. 404 with `known_issues` off. | AC-28.01, AC-28.02, AC-26.01 |
| GET | `/api/ui/projects/{slug}/tickets` | owner, super_admin, supervisor | `?stage=` and `?page=` | Ticket board: Triage, Backlog, In progress, Done. `tickets` (newest 25 by last change per column) and `counts` per stage. With `?stage=`, one page of that column as for the public board; an unknown stage is 404. | AC-25.01 |
| POST | `/api/ui/projects/{slug}/tickets` | owner, super_admin | `title`, `description`, `stage` (`triage`, `backlog` or `in_progress`) | The team files a ticket. No ref ID, no daily caps and no webhook. Rules for `title` and `description` as 8.2; a stage of `done` or another value is 422 on `stage`. 201 `{number}`. The ticket shows `filed_by`. | |
| GET | `/api/ui/projects/{slug}/tickets/{number}` | owner, super_admin, supervisor | | Full ticket (`TicketRow` above), including its description. | |
| PUT | `/api/ui/projects/{slug}/tickets/{number}/stage` | owner, super_admin | `stage`, `close_reason` (`fixed`, `wont_fix` or `duplicate`; required for `done`) | Move, 204. An unknown stage is 422 on `stage`. Missing reason for `done`: 422 on `close_reason`. Leaving `done` clears the reason. A move to another column sends the `ticket.moved` webhook in the same write (6.2). | AC-25.03 |
| PUT | `/api/ui/projects/{slug}/tickets/{number}/public-title` | owner, super_admin | `public_title` (up to 120 chars; empty clears it) | Save, 204. Longer is 422 on `public_title`. | AC-26.01 |
| POST | `/api/ui/projects/{slug}/tickets/{number}/revoke-ref` | owner, super_admin | | Revoke the ticket's ref ID, 204. Tickets stay. A team ticket has no ref ID: 404. | AC-27.01, AC-27.02 |
| POST | `/api/ui/projects/{slug}/ref-ids` | owner, super_admin | `single_use` (boolean, default false) | The team issues a ref ID from the board, for someone who reports outside the app. 201 `{ref_id, single_use, created_at}`: the raw ref ID, this once. Only its hash and last 4 characters are stored. | AC-23.01, AC-23.02 |

## 8.6 Tickets across projects

The React page `/tickets` calls this route (conventions 1.10). It lists tickets waiting in Triage. A TL sees the tickets of the projects they own; a super admin and a supervisor see every project.

| Method | Path | Access | Input | Behavior | AC |
| --- | --- | --- | --- | --- | --- |
| GET | `/api/ui/tickets` | session | Query `q`, `sort`, `page` | One page of 25. `tickets`: each `{number, title, created_at, project}` with `project` as in 3.1 (no `pulse`). `total`: the tickets that match, on every page. `page`, `page_size` (25). `q` matches the title, `#12` (a ticket number) or the project's name, ignoring case; it counts its first 100 characters. `sort` is `oldest` (the longest waiting first); anything else is newest first. `page` counts from 1; a missing or bad value reads page 1. | |
