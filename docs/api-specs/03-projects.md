# 3. Projects

Fields: [../technical-specs/06-data-model.md sections 6.3.4 and 6.3.5](../technical-specs/06-data-model.md#634-projects).

## 3.1 HTML routes

| Method | Path | Access | Input | Behavior | AC |
| --- | --- | --- | --- | --- | --- |
| GET | `/` | session | | Project list: own projects for a TL, all for a super admin | |
| POST | `/projects` | session | `name`, `slug` | Create, owner = caller. Taken slug: 422 "Slug already used". Seeds two `alert_routes` rows. | AC-06.01, AC-06.02 |
| GET | `/projects/{slug}` | owner, super_admin | | Overview: monitors, heartbeats, open incidents | |
| POST | `/projects/{slug}/owner` | super_admin | `owner_id` | Reassign. Data unchanged. | AC-05.01 to AC-05.03 |
| GET | `/projects/{slug}/keys` | owner, super_admin | | List keys: name, prefix, last used, revoked | |
| POST | `/projects/{slug}/keys` | owner, super_admin | `name` | Create. The page shows the full key once. | AC-07.01 |
| POST | `/projects/{slug}/keys/{id}/revoke` | owner, super_admin | | Set `revoked_at` | AC-07.02 |
| POST | `/projects/{slug}/allowlist` | owner, super_admin | `cidrs` (empty turns it off) | Validate each CIDR. Save. | AC-08.01, AC-08.02 |

## 3.2 API key rule for every JSON call

Keys belong to the project, not the owner. Disabling a TL leaves their projects and keys running; a super admin reassigns the projects.

| Condition | Result |
| --- | --- |
| No header or unknown key | 401 `unauthorized` |
| Revoked key | 401 `unauthorized` (AC-07.02) |
| Allowlist on and client IP outside it | 403 `forbidden` (AC-08.02) |
| Over 120 requests per minute for the key | 429 `rate_limited` |
