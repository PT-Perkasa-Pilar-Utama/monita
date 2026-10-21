# 3. Projects

Fields: [../technical-specs/06-data-model.md sections 6.3.4 and 6.3.5](../technical-specs/06-data-model.md#634-projects).

## 3.1 Frontend routes

The React pages `/`, `/projects/{slug}` and `/projects/{slug}/settings` with its `status-page`, `keys`, `secrets`, `allowlist` and `alerts` sub-pages call these routes (conventions 1.10). A project another tech lead owns is 404 on every route, reads and writes alike. A supervisor and a super admin see every project; a supervisor's writes are 403 (1.9).

A project as every route shows it (`project`):

| Field | Value |
| --- | --- |
| `name`, `slug`, `owner_email` | strings |
| `owner_id`, `owner_avatar_at` | the owner's id and when their picture last changed, or null. The picture is at `/users/{owner_id}/avatar?v={owner_avatar_at}` (2.5). |
| `description` | text up to 280 characters, or null |
| `homepage_url` | `http` or `https` address with no user name or password, or null for a project made before it was required |
| `icon_url` | `/projects/{slug}/icon?v={icon_at}`, or null |
| `ip_allowlist` | the CIDRs as saved, or null |
| `created_at` | Unix seconds |
| `pages` | `{incidents, known_issues, report, changelog}`: the public tabs (3.1, `public-pages`) |

| Method | Path | Access | Input | Behavior | AC |
| --- | --- | --- | --- | --- | --- |
| GET | `/api/ui/projects` | session | Query `page` (from 1; missing, bad or lower is 1), `q` (search, trimmed, first 100 characters), `owner` (an owner's email; blank for any), `sort` (`name`, `oldest`; anything else is newest first) | One page of 20: own projects for a TL, all for a super admin or a supervisor. `projects`, each a `project` plus `pulse`. `total`: the projects that match `q` and `owner` on every page. `page`, `page_size` (20). A page past the end has no `projects` and the real `total`. `q` matches name, slug, owner email or description, ignoring ASCII case. `owners`: who a super admin may name as owner (email, role); empty for everyone else. `owner_options`: the emails that own a project, for the Owner filter; empty for a TL or a supervisor. | |
| POST | `/api/ui/projects` | session | `name` (up to 80 chars), `homepage_url` (required), `description` (optional, up to 280 chars), `icon` (optional base64 PNG or WebP, up to 32 KB, scaled by the page), `owner_email` (super admin only; blank is the caller) | Create, 201 with the `project`. No project cap. A TL always owns what they create. Every bad field comes back at once in `error.fields`: `name`, `homepage_url`, `description`, `icon`; a super admin naming an unknown owner gets `owner_email`. `homepage_url` must start with `http://` or `https://` and hold at most 2 000 characters. The slug is made from the name plus 6 random characters; a taken draw is drawn again. Seeds two `alert_routes` rows. | AC-06.01, AC-06.02 |
| GET | `/api/ui/projects/{slug}` | owner, super_admin, supervisor | | `project` and `counts` for the tab bar: `monitors`, `heartbeats` (totals), `monitors_down`, `heartbeats_down`, `incidents` (open), `tickets` (not done) and `triage`. The tabs show a number only for what needs a look: down, open and Triage. | |
| DELETE | `/api/ui/projects/{slug}` | super_admin | `confirm` (the project's name, typed again) | Delete the project and everything in it: checks, incidents, tickets, keys, secrets, alert settings and releases. A `confirm` that is not the name is 422 on `confirm`. 204. A TL gets 403, whoever owns the project. | |
| GET | `/api/ui/projects/{slug}/overview` | owner, super_admin, supervisor | | `monitors` and `heartbeats` (`total`, worst `kind`, a sentence), `triage` (tickets in Triage), `incidents` (the newest 5 open), `active_keys`, and `timeline` (v1.1 Q39, below). | |
| PUT | `/api/ui/projects/{slug}/profile` | owner, super_admin | `description`, `homepage_url` (required), `icon`, `remove_icon` (`on`) | Saves the description and the homepage. A new icon replaces the old one; `remove_icon` without a new icon removes it. 204, or 422 with every bad field in `error.fields`. | AC-06.04 |
| GET | `/projects/{slug}/icon` | owner, super_admin, supervisor | `?v={icon_at}` | The icon with its stored type and `Cache-Control: private, max-age=31536000, immutable`. 404 when there is none. | AC-06.04 |
| GET | `/api/ui/projects/{slug}/owners` | super_admin | | Who may own the project: the acting super admin first, then every active TL (email, role). A supervisor is never listed. | AC-05.01 |
| PUT | `/api/ui/projects/{slug}/owner` | super_admin | `owner_email` | Reassign to the eligible owner with that email. Data unchanged. 204; an unknown email is 422 on `owner_email`. | AC-05.01 to AC-05.03 |
| GET | `/api/ui/projects/{slug}/keys` | owner, super_admin, supervisor | | The keys: `id`, `name`, `prefix`, `last_used_at`, `revoked_at`, `created_at`. Never the key itself. | AC-07.01, AC-07.02 |
| POST | `/api/ui/projects/{slug}/keys` | owner, super_admin | `name` (1 to 60 chars) | Create, 201 `{key}`: the full key, this once. A bad name is 422 on `name`; a project with 20 live keys is 422 with the cap in the message. | AC-07.01 |
| POST | `/api/ui/projects/{slug}/keys/{id}/revoke` | owner, super_admin | | Set `revoked_at`, 204. | AC-07.02 |
| PUT | `/api/ui/projects/{slug}/allowlist` | owner, super_admin | `cidrs` (one per line; empty turns it off) | Validate each CIDR. Save, 204, or 422 on `cidrs`. | AC-08.01, AC-08.02 |
| PUT | `/api/ui/projects/{slug}/public-pages` | owner, super_admin | `incidents`, `known_issues`, `report`, `changelog` (booleans) | Choose the public tabs besides Uptime, which always shows. A missing field means on, as for a new project (v1.1 Q28). 204. A tab off answers 404 on its public routes (7.2, 8.5, 11.2). `report` switches both the Report a bug form and the reporter's own tickets. | |

`timeline` holds `monitors` and `heartbeats`, each a list of lanes, at most 10 each. A lane is `{id, name, state, uptime, hours}`. `state` is `up`, `down`, `issue`, `maintenance`, `paused` or `unknown`. `uptime` is the share of the last 24 hours the source was watched and not down, or null when it was not watched. `hours` holds 24 entries `{day, uptime, downtime_s}`, oldest first, where `day` is the hour such as `14:00` in the server's time zone. They are the same hourly buckets as a project's `pulse`: downtime comes from automatic incidents, and an hour when the source was not watched (not yet created, or paused) has null `uptime`. Lanes sort by state (down, issue, maintenance, no data yet, up, paused), then the lowest `uptime`, then the name without regard to case.

`pulse` on a list row is the project at a glance:

| Field | Value |
| --- | --- |
| `kind` | `up`, `down`, `issue`, `maintenance`, `paused` (every one paused) or `unknown`: the worse of the monitors' and heartbeats' |
| `monitors`, `heartbeats` | `{total, kind, text}`: the count, the worst state and one sentence |
| `incidents`, `tickets` | open incidents; tickets not done |
| `hours` | 24 hourly bars ending with the current hour, oldest first: `day` (`14:00`, the hour's start in the admin's time zone), `uptime` (percent, or null when no monitor or heartbeat was watched that hour) and `downtime_s`. A source counts from when it was created until it was paused, so an hour before any existed, or while all are paused, has no data. Only automatic incidents count as downtime. |

## 3.2 API key rule for every JSON call

Keys belong to the project, not the owner. Disabling a TL leaves their projects and keys running; a super admin reassigns the projects.

| Condition | Result |
| --- | --- |
| No header or unknown key | 401 `unauthorized` |
| Revoked key | 401 `unauthorized` (AC-07.02) |
| Allowlist on and client IP outside it | 403 `forbidden` (AC-08.02) |
| Over 120 requests per minute for the key | 429 `rate_limited` |

## 3.3 Secrets

The page `/projects/{slug}/settings/secrets` holds the project's sealed secrets: tokens and passwords that a monitor sends (4.3). A value is written once. No response carries one, and no route reads one back. A project holds 20 secrets of up to 4 KB each. A supervisor may list (GET) and gets 403 on every write.

| Method | Path | Access | Input | Behavior | AC |
| --- | --- | --- | --- | --- | --- |
| GET | `/api/ui/projects/{slug}/secrets` | owner, super_admin, supervisor | | By name: `id`, `name`, `updated_at` and `used_by` (how many monitors authenticate with it). Never the value. | |
| POST | `/api/ui/projects/{slug}/secrets` | owner, super_admin | `name` (1 to 60 letters, digits, `_` or `-`), `value` (1 byte to 4 KB) | Create, 201 `{id}`. The value is not echoed. 422 on `name` for a bad or taken name, on `value` for a blank or long value; with 20 secrets 422 with the cap in the message and no field. | |
| PUT | `/api/ui/projects/{slug}/secrets/{id}` | owner, super_admin | `value` | Replace the value, 204. A bad value is 422 on `value`; an unknown id is 404. | |
| DELETE | `/api/ui/projects/{slug}/secrets/{id}` | owner, super_admin | | Delete, 204. Refused while a monitor uses it: 422 with the message "N monitors use it. Change them first." and no field. An unknown id is 404. | |
