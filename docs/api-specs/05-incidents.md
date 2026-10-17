# 5. Incidents

Rules: [../technical-specs/12-incident-engine.md](../technical-specs/12-incident-engine.md).

## 5.1 Frontend routes

The React pages under `/projects/{slug}/incidents` call these routes (conventions 1.10). A project another tech lead owns is 404 on every route. A refused form is 422 with the error on its field: `title`, `items`, `severity`, `message` or `label`. A cap error has no field.

| Method | Path | Access | Input | Behavior | AC |
| --- | --- | --- | --- | --- | --- |
| GET | `/api/ui/projects/{slug}/incidents` | owner, super_admin | `?state=resolved` (open when absent) | `incidents`: id, origin, title, severity, cause, start, end and the latest label. The page searches title and INC id and filters severity in the browser. | |
| GET | `/api/ui/projects/{slug}/incidents/{id}` | owner, super_admin | | `incident`, `items` (names of the affected monitors and heartbeats; a deleted one reads "Deleted monitor"), `updates` oldest first with label, message, author email (null for Monita) and time, and `labels`, the labels an update may take now. | AC-18.02 |
| GET | `/api/ui/projects/{slug}/incident-items` | owner, super_admin | | The project's monitors and heartbeats as `value` (`monitor:<id>` or `heartbeat:<id>`) and `name`, for the manual form. | AC-20.01 |
| POST | `/api/ui/projects/{slug}/incidents` | owner, super_admin | `title` (up to 120 chars), `items` (a list of `monitor:<id>` or `heartbeat:<id>`), `severity` (`critical` or `warning`, required), `message` (up to 10 000 chars) | Manual incident. First update has label `investigating`. Errors in form order: title, items, severity, message. 201 `{id}`. | AC-20.01 |
| POST | `/api/ui/projects/{slug}/incidents/{id}/updates` | owner, super_admin | `label` (`investigating`, `identified`, `monitoring`, `resolved`), `message` | Empty message: 422 (AC-19.02). After close, only `resolved` is accepted, as a follow-up note (AC-19.03); other labels are 422. `resolved` on a manual open incident closes it (AC-20.02). `resolved` on an open auto incident is 422: only a passing check closes it. 204 when stored. | AC-19.01 to AC-19.03, AC-20.02 |

The automatic open and close have no route. They come from the scheduler and `/ping` through `incidents::observe` (AC-17.01 to AC-18.01).
