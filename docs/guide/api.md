# Public API

Every way a project's own code talks to Monita: the status JSON, bug tickets, ref IDs and heartbeat pings. None of them needs a sign-in.

## At a glance

| Endpoint | Who can call it | What it does |
| --- | --- | --- |
| `GET /api/v1/public/status/{slug}` | Anyone, from any website | Public systems, daily uptime and incidents |
| `POST /api/v1/tickets` | Anyone with a ref ID, in the body | Files a bug report |
| `GET /api/v1/tickets/mine` | Anyone with a ref ID, in the `X-Ref-Id` header | Lists that reporter's tickets |
| `POST /api/v1/ref-ids` | Your server, with a project API key | Issues a ref ID for one of your users |
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
| `overall` | `up`, `down` or `issue`, for the whole page |
| `items[].state` | `up`, `down`, `issue` or `unknown` (no data yet) |
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

Store the ref ID with your user: Monita shows it only in this answer. A single-use ref ID files one ticket. A missing, unknown or revoked key is 401 `unauthorized`. The board's **Issue a ref ID** button does the same without code. See [Tickets](tickets.md#issue-a-ref-id).

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
{ "error": { "code": "rate_limited", "message": "Too many requests. Retry in 12 s." } }
```

| Limit | Default |
| --- | --- |
| Status JSON, tickets, wrong API keys and unknown ping tokens | 60 a minute per address |
| Calls with one API key | 120 a minute |
| Pings for one heartbeat | 1 a second |
| Request body | 128 KiB, except the ping routes |

A 429 carries `Retry-After` in seconds. A super admin changes the rates on [Admin and roles](admin.md#rate-limits).
