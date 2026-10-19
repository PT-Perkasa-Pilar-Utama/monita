# 5. Incidents

Rules: [../technical-specs/12-incident-engine.md](../technical-specs/12-incident-engine.md).

## 5.1 Frontend routes

The React pages under `/projects/{slug}/incidents` call these routes (conventions 1.10). A project another tech lead owns is 404 on every route. A supervisor may read and gets 403 on every write (conventions 1.9). A refused form is 422 with the error on its field: `title`, `items`, `severity`, `message` or `label`. A cap error has no field.

| Method | Path | Access | Input | Behavior | AC |
| --- | --- | --- | --- | --- | --- |
| GET | `/api/ui/projects/{slug}/incidents` | owner, super_admin, supervisor | `?state=resolved\|all` (open when absent or any other value) | `incidents`, the newest 200: `id`, `origin` (`auto` or `manual`), `title`, `severity`, `cause`, `started_at`, `resolved_at` (null while open) and `last_label`. `all` lists open and resolved together, ties broken by id. The page searches title and INC id and filters severity in the browser. | |
| GET | `/api/ui/projects/{slug}/incidents/{id}` | owner, super_admin, supervisor | | `incident`, `items` (names of the affected monitors and heartbeats; a deleted one reads "Deleted monitor"), `updates` oldest first with `label`, `message`, `author_email` (null for Monita) and `created_at`, and `labels`, the labels an update may take now: all four while open, only `resolved` after close. | AC-18.02 |
| GET | `/api/ui/projects/{slug}/incident-items` | owner, super_admin, supervisor | | The project's monitors and heartbeats, for the manual form: `value` (`monitor:<id>` or `heartbeat:<id>`), `kind` (`monitor` or `heartbeat`), `name` and `state` (`down`, `issue` or `maintenance` as the team's pages show it, or null when it is up). The maintenance form uses the same list. | AC-20.01 |
| POST | `/api/ui/projects/{slug}/incidents` | owner, super_admin | `title` (up to 120 chars), `items` (a list of `monitor:<id>` or `heartbeat:<id>`), `severity` (`critical` or `warning`, required), `message` (up to 10 000 chars) | Manual incident. First update has label `investigating`. Errors in form order: title, items, severity, message. A project with 20 open manual incidents is 422 with no field. 201 `{id}`. | AC-20.01 |
| POST | `/api/ui/projects/{slug}/incidents/{id}/updates` | owner, super_admin | `label` (`investigating`, `identified`, `monitoring`, `resolved`), `message` (1 to 10 000 chars) | Empty message: 422 on `message` (AC-19.02). After close, only `resolved` is accepted, as a follow-up note (AC-19.03); other labels are 422 on `label`. `resolved` on an open manual incident closes it (AC-20.02). `resolved` on an open auto incident also closes it, by hand: the resolved alert goes out ("marked resolved by a tech lead"), a heartbeat gets a new deadline from now, and the source starts over, so a monitor needs its full failure count again to open the next incident. An incident takes 50 updates; one more is 422 with no field. 204 when stored; an unknown incident is 404. | AC-19.01 to AC-19.03, AC-20.02 |

## 5.2 Incidents across projects

The React page `/incidents` calls this route (conventions 1.10). A TL sees the incidents of the projects they own; a super admin and a supervisor see every project.

| Method | Path | Access | Input | Behavior | AC |
| --- | --- | --- | --- | --- | --- |
| GET | `/api/ui/incidents` | session | Query `q`, `state`, `severity`, `sort`, `page` | One page of 25. `incidents`: each an incident as in 5.1 plus `project` (the project as in 3.1, without `pulse`). `total`: the incidents that match, on every page. `page`, `page_size` (25). `q` matches the title, the project's name, or `INC-12`, ignoring case; it counts its first 100 characters. `state` is `open` (default, also for any other value), `resolved` or `all`. `severity` is `critical` or `warning`; blank is both. `sort` is `newest`, `oldest`, or blank: critical first, then newest. `page` counts from 1; a missing or bad value reads page 1. | |

## 5.3 Maintenance windows

The React page `/projects/{slug}/incidents/maintenance` calls these routes (v1.1 decision Q18). Access is as in 5.1. A window's state follows from its times: `scheduled` before `starts_at`, `active` until `ends_at`, then `ended`. A window has no edit route.

| Method | Path | Access | Input | Behavior | AC |
| --- | --- | --- | --- | --- | --- |
| GET | `/api/ui/projects/{slug}/maintenance` | owner, super_admin, supervisor | `?page=` | One page of 20, latest start first. `windows`: `id`, `title`, `message`, `starts_at`, `ends_at`, `state`, and `items` (`value`, `kind`, `name` of each covered source that still exists). `total`, `page`, `page_size`. | #11 |
| POST | `/api/ui/projects/{slug}/maintenance` | owner, super_admin | `title` (1 to 120 chars), `message` (up to 1000 chars, shown on the status page), `starts_at` and `ends_at` (Unix seconds), `items` (as in 5.1) | A start in the past becomes now. The end must be in the future, after the start, and at most 30 days after it. Errors in form order: title, starts_at, ends_at, items, message. A project with 20 windows scheduled or active is 422 with no field. 201 `{id}`. | #11 |
| DELETE | `/api/ui/projects/{slug}/maintenance/{id}` | owner, super_admin | | Deletes a scheduled window, 204. An active or ended window is 422 with no field. An unknown window is 404. | #11 |
| POST | `/api/ui/projects/{slug}/maintenance/{id}/end` | owner, super_admin | | End now: an active window's end becomes now, 204. A scheduled or ended window is 422 with no field. An unknown window is 404. | #11 |

While a window is active, a covered source's failure opens no incident and sends no alert; its results are still stored. Spec 12 section 12.3a has the rule.

The automatic open and close have no route. They come from the scheduler and `/ping` through `incidents::observe` (AC-17.01 to AC-18.01).
