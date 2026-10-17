# Status page

Each project has a public status page at `/status/<slug>`. It shows the monitors and heartbeats marked public, their last 90 days, and the incidents that touched them.

## What users see

- A headline: everything is up, or which item is down.
- One row per public item with its state and 90 daily bars. A red bar had downtime; a grey bar is a day before the item existed.
- Incidents of the last 90 days, newest first, with their updates.

Users never see who posted an update, nor the technical cause Monita recorded. Manual incidents show as an issue, not downtime.

From the status page users can also read the known issues and report a bug. See [Tickets](tickets.md).

## Status JSON

Build your own status widget from the same data.

```sh
curl -s https://monita.example.com/api/v1/public/status/payments?days=30
```

| Part | Value |
| --- | --- |
| Access | Public, 60 requests per minute per IP, CORS `*`, cached 30 s |
| `days` | 1 to 90, default 90 |

```json
{
  "data": {
    "project": { "name": "Payments", "slug": "payments" },
    "overall": "down",
    "items": [
      { "kind": "monitor", "name": "API health", "state": "down", "uptime_90d": 99.95,
        "days": [ { "day": "2026-10-15", "uptime": 100.0, "downtime_s": 0 } ] }
    ],
    "incidents": [
      { "title": "API health is down", "origin": "auto", "severity": "critical",
        "started_at": 1792137600, "resolved_at": null, "items": ["API health"],
        "updates": [ { "label": "investigating", "message": "Detected.", "at": 1792137600 } ] }
    ]
  }
}
```

| Field | Values |
| --- | --- |
| `state` | `up`, `down`, `issue` (an open manual incident), `unknown` (no data yet) |
| `overall` | `down` if any item is down, else `issue` if any has an issue, else `up` |
| `days[].uptime` | `null` for a day before the item existed |
