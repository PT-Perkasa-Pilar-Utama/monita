# API

Every way a project's own code talks to Monita: the status JSON, bug tickets, ref IDs, heartbeat pings, and the key API that manages a project. None of them needs a sign-in.

## At a glance

| Endpoint | Who can call it | What it does |
| --- | --- | --- |
| `GET /api/v1/public/status/{slug}` | Anyone, from any website | Public systems, daily uptime and incidents |
| `POST /api/v1/tickets` | Anyone with a ref ID, in the body | Files a bug report |
| `GET /api/v1/tickets/mine` | Anyone with a ref ID, in the `X-Ref-Id` header | Lists that reporter's tickets |
| `POST /api/v1/ref-ids` | Your server, with a project API key | Issues a ref ID for one of your users |
| `/api/v1/projects/{slug}/...` | Your scripts, with a read or write key | Reads or changes the project's monitors, heartbeats, incidents and maintenance |
| `GET`, `HEAD` or `POST /ping/{token}` | Whoever holds the heartbeat URL | Reports that a job succeeded |
| `POST /ping/{token}/fail` | Whoever holds the heartbeat URL | Reports that a job failed, with a reason |

Replace `monita.example.com` in the examples with your Monita's address (`MONITA_BASE_URL`). Each project's **Settings**, **Status page** shows the same examples with its own address and slug.

## Status JSON

Read the systems you mark as public, their days and the incidents. A browser may call it from any website (`Access-Control-Allow-Origin: *`).

```sh
curl -s "https://monita.example.com/api/v1/public/status/payments?days=90"
```

```json
{
  "data": {
    "project": { "name": "Payments", "slug": "payments" },
    "overall": "down",
    "items": [
      {
        "id": 2,
        "kind": "monitor",
        "name": "API health",
        "state": "down",
        "uptime_90d": 99.95,
        "days": [
          { "day": "2026-10-18", "uptime": 100, "downtime_s": 0 },
          { "day": "2026-10-19", "uptime": 98.6, "downtime_s": 1210 }
        ]
      }
    ],
    "incidents": [
      {
        "title": "API health is down",
        "origin": "auto",
        "severity": "critical",
        "started_at": 1792396936,
        "resolved_at": null,
        "items": ["API health"],
        "updates": [
          { "label": "identified", "message": "Database failover in progress.", "at": 1792397236 },
          { "label": "investigating", "message": "Detected.", "at": 1792396936 }
        ]
      }
    ]
  }
}
```

| Field | Meaning |
| --- | --- |
| `days` (query) | 1 to 90, default 90. How many days of bars come back, and how far back incidents go. Open incidents always come back. |
| `overall` | `up`, `down`, `issue` or `maintenance` for the whole page, or `unknown` while no public system was checked |
| `items[].state` | `up`, `down`, `issue`, `maintenance` or `unknown` (no data yet). A paused system is left out. |
| `items[].days[]` | Oldest first. `uptime` is `null` for a day before the system existed. |
| `incidents[].updates[]` | Newest first. `label` is `investigating`, `identified`, `monitoring` or `resolved`. |
| Times | Unix seconds |

While the project's Incidents tab is off, `incidents` is empty. An answer can be up to 30 seconds old.

### A page of your own

**Settings**, **Status page**, **Custom status page** has **Download template**: one HTML file that reads this JSON, draws your systems and incidents, and reloads every minute. Host it anywhere, or open it from disk.

## File a ticket

Your page or app files a bug report for a user who has a ref ID. It lands in **Triage** on the project's ticket board.

```sh
curl -s -X POST "https://monita.example.com/api/v1/tickets" \
  -H "Content-Type: application/json" \
  -d '{"ref_id": "rf_5k2m9q7w3x8p1t6v4y0z2a8bcd", "title": "Export button does nothing", "description": "I click Export on the March report and nothing downloads.", "page_url": "https://app.example.com/reports/march"}'
```

```json
{ "data": { "number": 17, "stage": "triage", "created_at": 1792137600 } }
```

| Field | Rule |
| --- | --- |
| `ref_id` | Required |
| `title` | Required, 3 to 120 characters |
| `description` | Required, 1 to 10 000 characters |
| `page_url` | Optional, an `http` or `https` address |

A ref ID files 5 tickets a day, and a project takes 100 a day. Past either, the answer is 429 `daily_cap_reached`. An unknown, revoked or used single-use ref ID is 422 `ref_id_invalid`.

## A reporter's tickets

List the tickets filed with one ref ID, newest first. Send the ref ID in a header, so it stays out of logs.

```sh
curl -s "https://monita.example.com/api/v1/tickets/mine" -H "X-Ref-Id: rf_5k2m9q7w3x8p1t6v4y0z2a8bcd"
```

```json
{
  "data": [
    { "number": 17, "title": "Export button does nothing", "stage": "in_progress", "close_reason": null, "created_at": 1792137600, "updated_at": 1792224000 }
  ]
}
```

`stage` is `triage`, `backlog`, `in_progress` or `done`. `close_reason` is `fixed`, `wont_fix` or `duplicate` once a ticket is done. The description never comes back.

## Issue a ref ID

Your server gives one of your users a ref ID. Use a project API key from **Settings**, **API keys**. Never put the key in a browser page.

```sh
curl -s -X POST "https://monita.example.com/api/v1/ref-ids" \
  -H "Authorization: Bearer $MONITA_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"single_use": false}'
```

```json
{ "data": { "ref_id": "rf_5k2m9q7w3x8p1t6v4y0z2a8bcd", "single_use": false, "created_at": 1792137600 } }
```

Store the ref ID with your user: Monita shows it only in this answer. A single-use ref ID files one ticket. A missing, unknown or revoked key is 401 `unauthorized`. A read key gets 403 here. Use a **Ref IDs only** or a **Read and write** key. The board's **Issue a ref ID** button does the same without code. See [Tickets](tickets.md#issue-a-ref-id).

## Manage a project

A script or a deploy pipeline manages its own project with a project API key. Pick the key's scope when you create it in **Settings**, **API keys**:

| Scope | What the key may do |
| --- | --- |
| Ref IDs only | Issue ref IDs. Every key made before v1.1 has this scope. |
| Read | Read the project, its monitors, heartbeats, incidents and maintenance. It never sees a ping URL, a header value or a request body. |
| Read and write | Also create, change, pause and delete monitors and heartbeats, open incidents, post updates and plan maintenance. It issues ref IDs too. |

The routes are the ones the team's pages use, with `/api/v1/projects/{slug}` in front:

| Path after `/api/v1/projects/{slug}` | What it does |
| --- | --- |
| (nothing), `/overview` | The project and its numbers |
| `/monitors`, `/monitors/{id}` | List, create, read, change and delete monitors |
| `/heartbeats`, `/heartbeats/{id}` | The same for heartbeats |
| `/monitors/{id}/paused`, `/heartbeats/{id}/paused` | `PUT {"paused": true}` pauses, `false` resumes |
| `/monitors/batch`, `/heartbeats/batch` | `POST {"action": "pause", "ids": [3, 4]}` pauses, resumes or deletes up to 100 at once |
| `/incidents`, `/incidents/{id}`, `/incidents/{id}/updates` | List and open incidents, and post updates |
| `/incidents/{id}/class` | `PUT {"class": "false_alarm"}` marks an incident planned work or a false alarm, or counts it again |
| `/maintenance`, `/maintenance/{id}`, `/maintenance/{id}/end` | List, plan, delete and end maintenance windows |

Pause a monitor while you deploy, then resume it:

```sh
curl -s -X PUT "https://monita.example.com/api/v1/projects/payments/monitors/3/paused" \
  -H "Authorization: Bearer $MONITA_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"paused": true}'
```

Post an update to an open incident:

```sh
curl -s -X POST "https://monita.example.com/api/v1/projects/payments/incidents/12/updates" \
  -H "Authorization: Bearer $MONITA_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"label": "monitoring", "message": "The rollback is out, and we are watching the error rate."}'
```

The team sees the key's name as the update's author. The public page shows no author.

| Answer | When |
| --- | --- |
| 401 `unauthorized` | No key, an unknown key or a revoked key |
| 403 `forbidden` | A Ref IDs only key, a read key that tries to change something, or a caller outside the allowlist |
| 404 `not_found` | Another project's slug, or a route the key API does not have, such as keys, secrets, alert cards or tickets |
| 405 | A method the route does not take, such as `DELETE` on the project itself |
| 422 `validation_failed` | A refused form, with each bad field in `error.fields` |

Fields may be added to an answer within v1, but none is removed or renamed. Ignore fields you do not know. The full contract is in `docs/api-specs/03-projects.md` section 3.4.

## Heartbeat pings

A scheduled job calls its heartbeat URL when it finishes. The token in the URL is the heartbeat's whole secret.

```sh
# Success: GET, HEAD or POST. Monita ignores the body.
curl -fsS -m 10 https://monita.example.com/ping/<token>

# Failure: Monita opens an incident at once and keeps the first 1 KB of the body as the cause.
curl -fsS -m 10 --data-binary "disk full on /backup" https://monita.example.com/ping/<token>/fail
```

Both answer `OK` as plain text. An unknown token is 404. One heartbeat takes 1 ping a second by default; `/fail` has its own limit. More in [Heartbeats](heartbeats.md#rules).

## Webhooks

Alerts can post JSON to a chat or any URL you set. The bodies and their `event` values are on [Alerts](alerts.md#what-recipients-get).

## Errors and limits

Every error has the same shape, with its HTTP status:

```json
{ "error": { "code": "rate_limited", "message": "Too many requests, so retry in 12 s." } }
```

| Limit | Default |
| --- | --- |
| Calls with one API key | 120 a minute |
| Pings for one heartbeat | 1 a second |
| Request body | 128 KiB, except the ping routes |

A 429 carries `Retry-After` in seconds. A super admin changes the rates on [Admin and roles](admin.md#rate-limits).
