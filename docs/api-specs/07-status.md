# 7. Status

## 7.1 `GET /api/v1/public/status/{slug}`

| Part | Contract |
| --- | --- |
| Purpose | Give a project the data for its own status page. |
| Access | public, 60/min per IP, CORS `*`, `Cache-Control: max-age=30` |
| Input | Path `slug`. Query `days` 1 to 90, default 90. Any other value is 400 `invalid_request`. |
| Behavior | Load public monitors and heartbeats only (AC-21.02). Current state from `incidents::states_for`. Daily uptime from the overlap of automatic incidents with each UTC day (manual incidents are not downtime), computed live. `daily_summaries` is the archival record after raw rows are swept (Q12, AC-32.01); no page reads it in v1. Incidents that touch a public item and started within `days` or are still open, newest first; their item lists show public items only. Updates carry label, message and time, never the author. The JSON is built from wire types that live in `status` and are not shared with `incidents`, so a field added on the incident side reaches the public JSON only by an edit in `status`; Monita's own detection entry reads `Detected.` without its cause (SEC-21). (AC-22.01) |
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

## 7.2 Frontend routes

The React pages `/status/{slug}` and `/status/{slug}/history` call these routes with no sign-in. Both carry the 7.1 wire types, so neither sends an author or Monita's cause text (SEC-21). Both are `Cache-Control: public, max-age=30`, and the per-IP limit covers `/api/ui/public/` and the pages under `/status/`.

| Method | Path | Access | Behavior | AC |
| --- | --- | --- | --- | --- |
| GET | `/api/ui/public/{slug}/status` | public, 60/min per IP | `status`: the project, `overall`, every public item with 90 days, and every open incident however old, with its updates. `recent`: the newest 10 incidents resolved in the last 7 days. The page shows two columns from 1000 px: systems on the left, incidents on the right; one column below, with active incidents first. The headline counts systems up. Systems with a problem come first; they and the first 12 others show 90 daily bars, the rest a plain list. Resolved incidents are folded. | AC-06.03, AC-21.01, AC-21.02 |
| GET | `/api/ui/public/{slug}/history` | public, 60/min per IP | `?before=<started_at>.<id>` from `older`. `incidents`: every incident that touched a public system, newest first, 25 per page; `older` is the next cursor or null. The page groups them under month headings. | AC-22.01 |
