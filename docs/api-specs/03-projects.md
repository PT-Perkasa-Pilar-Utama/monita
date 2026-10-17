# 3. Projects

Fields: [../technical-specs/06-data-model.md sections 6.3.4 and 6.3.5](../technical-specs/06-data-model.md#634-projects).

## 3.1 HTML routes

| Method | Path | Access | Input | Behavior | AC |
| --- | --- | --- | --- | --- | --- |
| GET | `/` | session | `?q=` (name, slug, owner email or description) | Project cards: own projects for a TL, all for a super admin. The search narrows that list only. | |
| POST | `/projects` | session | `name`, `description` (optional, up to 280 chars), `icon` (optional base64 PNG or WebP, up to 32 KB, scaled by the page), `owner_email` (super admin only; blank is the caller) | Create. A TL always owns what they create; an unknown owner, a long description or a bad icon is 422 with the dialog open. The slug is made from the name plus 6 random characters; a taken draw is drawn again. A blank name: 422. Seeds two `alert_routes` rows. Redirects to the monitors page. | AC-06.01, AC-06.02 |
| GET | `/projects/{slug}` | owner, super_admin | | Overview: a tile per area (monitors, heartbeats, tickets in Triage), the newest 5 open incidents, and the project's facts and counts. | |
| GET | `/projects/{slug}/settings` | owner, super_admin | | General: icon, name, description, slug, status page, created, owner. Edit project and, for a super admin, Reassign open dialogs here. | AC-05.01 |
| GET | `/projects/{slug}/icon` | owner, super_admin | `?v={icon_at}` | The icon with its stored type and `Cache-Control: private, max-age=31536000, immutable`. 404 when there is none. | AC-06.04 |
| POST | `/projects/{slug}/profile` | owner, super_admin | `description`, `icon`, `remove_icon` (`on`) | Saves the description. A new icon replaces the old one; `remove_icon` without a new icon removes it. 422 with the dialog open on a bad value. Redirects to `/settings`. | AC-06.04 |
| GET | `/projects/{slug}/settings/keys` | owner, super_admin | | The key list (name, prefix, last used, revoked) and the New API key dialog. | AC-07.01, AC-07.02 |
| GET | `/projects/{slug}/settings/allowlist` | owner, super_admin | | The allowlist form. | AC-08.01 |
| POST | `/projects/{slug}/owner` | super_admin | `owner_email` | Reassign to the eligible owner with that email. Data unchanged. Redirects to `/settings`. An unknown email: 422 with the dialog open. | AC-05.01 to AC-05.03 |
| POST | `/projects/{slug}/keys` | owner, super_admin | `name` | Create. Answers with the keys page, which shows the full key once. A bad name re-renders it with the dialog open (422). | AC-07.01 |
| POST | `/projects/{slug}/keys/{id}/revoke` | owner, super_admin | | Set `revoked_at`. Redirects to `/settings/keys`. | AC-07.02 |
| POST | `/projects/{slug}/allowlist` | owner, super_admin | `cidrs` (empty turns it off) | Validate each CIDR. Save. Redirects to `/settings/allowlist`. | AC-08.01, AC-08.02 |

## 3.2 API key rule for every JSON call

Keys belong to the project, not the owner. Disabling a TL leaves their projects and keys running; a super admin reassigns the projects.

| Condition | Result |
| --- | --- |
| No header or unknown key | 401 `unauthorized` |
| Revoked key | 401 `unauthorized` (AC-07.02) |
| Allowlist on and client IP outside it | 403 `forbidden` (AC-08.02) |
| Over 120 requests per minute for the key | 429 `rate_limited` |
