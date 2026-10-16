# 5. Incidents

Rules: [../technical-specs/12-incident-engine.md](../technical-specs/12-incident-engine.md).

## 5.1 HTML routes

| Method | Path | Access | Input | Behavior | AC |
| --- | --- | --- | --- | --- | --- |
| GET | `/projects/{slug}/incidents` | owner, super_admin | `?state=open\|resolved` | List: title, severity, origin, start, duration | |
| GET | `/projects/{slug}/incidents/{id}` | owner, super_admin | | Detail: start, end, duration, cause, affected items, timeline | AC-18.02 |
| GET | `/projects/{slug}/incidents/new` | owner, super_admin | | Manual incident form: title, affected items, severity, first update | AC-20.01 |
| POST | `/projects/{slug}/incidents` | owner, super_admin | `title`, `items[]` (`monitor:<id>` or `heartbeat:<id>`), `message` | Manual incident. First update has label `investigating`. | AC-20.01 |
| POST | `/projects/{slug}/incidents/{id}/updates` | owner, super_admin | `label` (`investigating`, `identified`, `monitoring`, `resolved`), `message` | Empty message: 422 (AC-19.02). Allowed after close (AC-19.03). `resolved` on a manual open incident closes it (AC-20.02). `resolved` on an auto incident adds an entry only. | AC-19.01 to AC-19.03, AC-20.02 |

The automatic open and close have no route. They come from the scheduler and `/ping` through `incidents::observe` (AC-17.01 to AC-18.01).
