# 7. Status

## 7.1 `GET /api/v1/public/status/{slug}`

| Part | Contract |
| --- | --- |
| Purpose | Give a project the data for its own status page. |
| Access | public, 60/min per IP, CORS `*`, `Cache-Control: max-age=30` |
| Input | Path `slug`. Query `days` 1 to 90, default 90. Any other value is 400 `invalid_request`. |
| Behavior | Load public monitors and heartbeats only (AC-21.02). Current state from `incidents::states_for`. Daily uptime from the overlap of automatic incidents with each UTC day (manual incidents are not downtime), computed live. `daily_summaries` is the archival record after raw rows are swept (Q12, AC-32.01); no page reads it in v1. Incidents that touch a public item and started within `days` or are still open, newest first; their item lists show public items only. Updates carry label, message and time, never the author; Monita's own detection entry reads `Detected.` without its cause (SEC-21). (AC-22.01) |
| Errors | 404 `not_found` for an unknown slug. 429 `rate_limited`. |

Output 200:

```json
{
  "data": {
    "project": { "name": "Payments", "slug": "payments" },
    "overall": "down",
    "items": [
      {
        "kind": "monitor",
        "name": "API health",
        "state": "down",
        "uptime_90d": 99.95,
        "days": [ { "day": "2026-10-15", "uptime": 100.0, "downtime_s": 0 } ]
      }
    ],
    "incidents": [
      {
        "title": "API health is down",
        "origin": "auto",
        "severity": "critical",
        "started_at": 1792137600,
        "resolved_at": null,
        "items": ["API health"],
        "updates": [
          { "label": "identified", "message": "Database failover in progress.", "at": 1792137900 },
          { "label": "investigating", "message": "Detected.", "at": 1792137600 }
        ]
      }
    ]
  }
}
```

| Field | Values |
| --- | --- |
| `state` | `up`, `down`, `issue` (open manual incident), `unknown` (no data yet) |
| `overall` | `down` if any item is down, else `issue` if any has an issue, else `up` |
| `days[].uptime` | `null` for a day before the item existed. The page renders a grey bar. |

## 7.2 HTML routes

| Method | Path | Access | Behavior | AC |
| --- | --- | --- | --- | --- |
| GET | `/status/{slug}` | public | The same data as 7.1, rendered. Links to `/status/{slug}/tickets`. | AC-06.03, AC-21.01, AC-21.02 |
