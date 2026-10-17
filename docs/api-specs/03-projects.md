# 3. Projects

Fields: [../technical-specs/06-data-model.md sections 6.3.4 and 6.3.5](../technical-specs/06-data-model.md#634-projects).

## 3.1 HTML routes

| Method | Path | Access | Input | Behavior | AC |
| --- | --- | --- | --- | --- | --- |
| GET | `/` | session | | Project list: own projects for a TL, all for a super admin | |
| POST | `/projects` | session | `name`, `slug` | Create, owner = caller. Taken slug: 422 "Slug already used". Seeds two `alert_routes` rows. | AC-06.01, AC-06.02 |
| GET | `/projects/{slug}` | owner, super_admin | | Overview: a tile per area (monitors, heartbeats, tickets in Triage), the newest 5 open incidents, and the project's facts and counts. | |
| GET | `/projects/{slug}/settings` | owner, super_admin | | General: name, slug, status page, created, owner. A super admin reassigns from a dialog here. | AC-05.01 |
| GET | `/projects/{slug}/settings/keys` | owner, super_admin | | The key list (name, prefix, last used, revoked) and the New API key dialog. | AC-07.01, AC-07.02 |
| GET | `/projects/{slug}/settings/allowlist` | owner, super_admin | | The allowlist form. | AC-08.01 |
| POST | `/projects/{slug}/owner` | super_admin | `owner_id` | Reassign. Data unchanged. Redirects to `/settings`. | AC-05.01 to AC-05.03 |
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
