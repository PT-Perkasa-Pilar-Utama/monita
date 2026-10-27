# 7. Status

## 7.1 `GET /api/v1/public/status/{slug}`

| Part | Contract |
| --- | --- |
| Purpose | Give a project the data for its own status page. |
| Access | public, CORS `*`, `Cache-Control: max-age=30` |
| Input | Path `slug`. Query `days` 1 to 90, default 90. Any other value is 400 `invalid_request`. |
| Behavior | Load public monitors and heartbeats only (AC-21.02), and leave out any that a tech lead has paused (v1.1 Q38) until it resumes. Current state from `incidents::states_for`. Daily uptime from the overlap of automatic incidents with each day (manual incidents are not downtime), computed live. A day runs midnight to midnight in the admin's time zone offset (4.4); the default offset is UTC. `daily_summaries` is the archival record after raw rows are swept (Q12, AC-32.01); the month chart reads it (4.3). Incidents that touch a public item and started within `days` or are still open, newest first, at most 200; their item lists show public items only. With the project's Incidents tab off, `incidents` is empty (3.1, `public-pages`). Updates carry label, message and time, never the author. The JSON is built from wire types that live in `status` and are not shared with `incidents`, so a field added on the incident side reaches the public JSON only by an edit in `status`; Monita's own detection entry reads `Detected.` without its cause (SEC-21). (AC-22.01) |
| Errors | 404 `not_found` for an unknown slug. No rate limit applies (v1.1 Q45). |

Output 200:

```json
{
  "data": {
    "project": { "name": "Payments", "slug": "payments" },
    "overall": "down",
    "items": [
      {
        "kind": "monitor",
        "id": 7,
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
    ],
    "maintenance": [
      {
        "title": "Database upgrade",
        "message": "Checkout pauses until 02:00.",
        "starts_at": 1792170000,
        "ends_at": 1792173600,
        "items": ["API health"]
      }
    ]
  }
}
```

| Field | Values |
| --- | --- |
| `id` | The item's id. With `kind` it picks the item on the past-uptime route (7.2). It is not secret. |
| `state` | `up`, `down`, `issue` (open manual incident), `maintenance` (an active maintenance window covers it and no incident names it), `unknown` (no data yet) |
| `overall` | `down` if any item is down, else `issue` if any has an issue, else `maintenance` if any is in maintenance, else `up` if any item was checked, else `unknown` (no public item, or none checked yet) |
| `maintenance` | The active maintenance windows that cover a public item, soonest end first: `title`, `message` (may be empty), `starts_at`, `ends_at`, and the public `items` it covers. A window over hidden items only is left out. (v1.1 Q18) |
| `days[].day` | `YYYY-MM-DD` in the admin's time zone offset |
| `days[].uptime` | `null` for a day before the item existed. The page renders a grey bar. |

## 7.2 Frontend routes

The React pages under `/status/{slug}` call these routes with no sign-in. The page has an Uptime tab (always shown) and tabs for Incidents, Known issues, Report a bug and Changelog; a project switches the last four with `public-pages` (3.1). A route of a tab that is off answers 404, so the page and the route agree. The Uptime tab, `/status/{slug}/uptime` and the Incidents tab show the 7.1 wire types, so none sends an author or Monita's cause text (SEC-21). All are `Cache-Control: public, max-age=30`. No limit applies per IP address (Q45). The page's Refresh button skips that cache.

| Method | Path | Access | Behavior | AC |
| --- | --- | --- | --- | --- |
| GET | `/api/ui/public/{slug}/status` | public | `status`: the project, `overall`, every public item with 90 daily bars, and every open incident however old, with its updates (`incidents`). `recent`: the newest 10 incidents resolved in the last 7 days. With the Incidents tab off, both lists of incidents are empty. The Uptime tab shows the systems as a two-column grid of cards (name, state, 90 bars, uptime) and links the banner to open incidents. Each active window in `maintenance` shows as a note under the banner. Systems with a problem come first, then those in maintenance. | AC-06.03, AC-21.01, AC-21.02 |
| GET | `/api/ui/public/{slug}/uptime` | public | Query `kind` (`monitor` or `heartbeat`) and `id` together, or neither; `to` (a Unix time inside the last month wanted; a bad value reads now). Three months of one public system, a square per day. `project`; `systems`, every public system for the picker (`kind`, `id`, `name`); `system`, the one shown (the first when `kind` and `id` are absent; null when the project has no public system); `months`, oldest first: `month` (`2026-10`), `uptime` (percent over the days with data, or null) and `days` (as `days` in 7.1). `prev_to` and `next_to` are a `to` for the window before and after, or null at the ends: no window starts before the system was created or ends after the current month. A system that is not public, a half pair, a bad `kind` or a bad `id` is 404. | |
| GET | `/api/ui/public/{slug}/incidents` | public | Query `to` as above. Three months of public incidents, newest month first. `project`; `months`: `month` (`2026-10`), `incidents` (the wire type of 7.1, newest first, 25 at most) and `more` (a `before` cursor for the history route below, set when the month holds a full page of 25, else null); `prev_to` and `next_to` as for `uptime`. 404 with the Incidents tab off. | |
| GET | `/api/ui/public/{slug}/history` | public | `?before=<started_at>.<id>` from `older`. `project`; `incidents`: every incident that touched a public system, newest first, 25 per page; `older` is the next cursor or null. 404 with the Incidents tab off. | AC-22.01 |

The months follow the admin's time zone offset (4.4).
