# 3. Projects

Fields: [../technical-specs/06-data-model.md sections 6.3.4 and 6.3.5](../technical-specs/06-data-model.md#634-projects).

## 3.1 Frontend routes

The React pages `/`, `/projects/{slug}`, `/projects/{slug}/settings`, `/settings/keys`, `/settings/allowlist` and `/alerts` call these routes (conventions 1.10). A project another tech lead owns is 404 on every route, reads and writes alike.

| Method | Path | Access | Input | Behavior | AC |
| --- | --- | --- | --- | --- | --- |
| GET | `/api/ui/projects` | session | | `projects`: own projects for a TL, all for a super admin, each with `name`, `slug`, `owner_email`, `description`, `icon_url`, `ip_allowlist`, `created_at`. `owners`: who a super admin may name as owner (email, role); empty for a TL. The page's search filters this list in the browser. | |
| POST | `/api/ui/projects` | session | `name`, `description` (optional, up to 280 chars), `icon` (optional base64 PNG or WebP, up to 32 KB, scaled by the page), `owner_email` (super admin only; blank is the caller) | Create, 201 with the project. A TL always owns what they create; an unknown owner, a long description or a bad icon is 422 on that field. The slug is made from the name plus 6 random characters; a taken draw is drawn again. A blank name: 422 on `name`. Seeds two `alert_routes` rows. | AC-06.01, AC-06.02 |
| GET | `/api/ui/projects/{slug}` | owner, super_admin | | `project` (as in the list) and `counts`: monitors, heartbeats, open incidents, open tickets, for the tab bar. | |
| GET | `/api/ui/projects/{slug}/overview` | owner, super_admin | | `monitors` and `heartbeats` (`total`, worst `kind`, a sentence), `triage` (tickets in Triage), `incidents` (the newest 5 open), `active_keys`. | |
| PUT | `/api/ui/projects/{slug}/profile` | owner, super_admin | `description`, `icon`, `remove_icon` (`on`) | Saves the description. A new icon replaces the old one; `remove_icon` without a new icon removes it. 204, or 422 on the field. | AC-06.04 |
| GET | `/projects/{slug}/icon` | owner, super_admin | `?v={icon_at}` | The icon with its stored type and `Cache-Control: private, max-age=31536000, immutable`. 404 when there is none. | AC-06.04 |
| GET | `/api/ui/projects/{slug}/owners` | super_admin | | Who may own the project: the acting super admin first, then every active TL (email, role). | AC-05.01 |
| PUT | `/api/ui/projects/{slug}/owner` | super_admin | `owner_email` | Reassign to the eligible owner with that email. Data unchanged. 204; an unknown email is 422 on `owner_email`. | AC-05.01 to AC-05.03 |
| GET | `/api/ui/projects/{slug}/keys` | owner, super_admin | | The keys: `id`, `name`, `prefix`, `last_used_at`, `revoked_at`, `created_at`. Never the key itself. | AC-07.01, AC-07.02 |
| POST | `/api/ui/projects/{slug}/keys` | owner, super_admin | `name` | Create, 201 `{key}`: the full key, this once. A bad name is 422 on `name`; a full project is 422 with the cap in the message. | AC-07.01 |
| POST | `/api/ui/projects/{slug}/keys/{id}/revoke` | owner, super_admin | | Set `revoked_at`, 204. | AC-07.02 |
| PUT | `/api/ui/projects/{slug}/allowlist` | owner, super_admin | `cidrs` (one per line; empty turns it off) | Validate each CIDR. Save, 204, or 422 on `cidrs`. | AC-08.01, AC-08.02 |

## 3.2 API key rule for every JSON call

Keys belong to the project, not the owner. Disabling a TL leaves their projects and keys running; a super admin reassigns the projects.

| Condition | Result |
| --- | --- |
| No header or unknown key | 401 `unauthorized` |
| Revoked key | 401 `unauthorized` (AC-07.02) |
| Allowlist on and client IP outside it | 403 `forbidden` (AC-08.02) |
| Over 120 requests per minute for the key | 429 `rate_limited` |
