# 11. Changelog

A project lists its releases here. The team writes them; the public Changelog tab shows them, one release at a time, newest first. Table: [../technical-specs/06-data-model.md](../technical-specs/06-data-model.md). The tab starts off on every project and a project turns it on with `public-pages` (3.1).

A release is a `version`, a `released_on` day, a `title`, and four kinds of change in a fixed order: Added, Changed, Fixed, Removed. Each kind holds one change per line.

## 11.1 Team routes

The React page `/projects/{slug}/changelog` calls these routes (conventions 1.10). A project another tech lead owns is 404 on every route. A supervisor may read (GET) and gets 403 on every write (conventions 1.9).

| Method | Path | Access | Input | Behavior | AC |
| --- | --- | --- | --- | --- | --- |
| GET | `/api/ui/projects/{slug}/changelog` | owner, super_admin, supervisor | Query `q` (matches the version or the title, ignoring case), `sort` (`oldest`; anything else is newest first), `page` (from 1) | `releases`, 25 a page, newest release day first unless `sort` is `oldest`: each `{id, version, released_on, title, sections}`. `sections` is a list of `{tag, items}` for the kinds that have changes, in the order Added, Changed, Fixed, Removed. `total`: the releases that match, on every page. `page`, `page_size` (25). | |
| POST | `/api/ui/projects/{slug}/changelog` | owner, super_admin | the release form (below) | Create, 201 `{id}`. | |
| PUT | `/api/ui/projects/{slug}/changelog/{id}` | owner, super_admin | the release form | Replace the release, 204. An unknown id is 404. | |
| DELETE | `/api/ui/projects/{slug}/changelog/{id}` | owner, super_admin | | Delete, 204. | |

The release form:

| Field | Rule |
| --- | --- |
| `version` | 1 to 40 characters: letters, digits, `.`, `_` or `-`, starting with a letter or digit. `latest` is taken (11.2). Unique in the project: a repeat is 422 on `version`. |
| `released_on` | `YYYY-MM-DD`, a real day |
| `title` | 1 to 120 characters |
| `added`, `changed`, `fixed`, `removed` | One change per line, at most 30 lines of 500 characters each. A leading `-` or `*` is dropped, and blank lines are skipped. At least one of the four must hold a change; if none does, 422 on `added`. |

A refused form is 422 with every bad field in `error.fields`.

## 11.2 Public routes

The public page `/status/{slug}/changelog` calls these routes with no sign-in. Both answer `Cache-Control: public, max-age=30`. Both answer 404 while the project's Changelog tab is off, or for an unknown slug. They return only what the team typed: version, day, title and changes.

| Method | Path | Access | Input | Behavior | AC |
| --- | --- | --- | --- | --- | --- |
| GET | `/api/ui/public/{slug}/changelog` | public | Query `q` (version or title), `before=<released_on>.<id>` from `older` | The version index. `releases`: each `{version, title, released_on}`, newest first, 25 a page. `older` is the next cursor, or null. A bad cursor reads the first page. | |
| GET | `/api/ui/public/{slug}/changelog/{version}` | public | | One release. `release`: `{version, released_on, title, sections}`. `newer` and `older`: the neighbouring releases as `{version, title, released_on}`, or null at the ends. The version `latest` names the newest release. An unknown version, or `latest` with no release, is 404. | |
