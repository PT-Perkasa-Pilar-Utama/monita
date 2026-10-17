# 2. Authentication

Session strategy: [../technical-specs/07-security.md](../technical-specs/07-security.md) SEC-01 to SEC-03, SEC-06, SEC-11.

## 2.1 CLI: `monita admin create`

| Part | Contract |
| --- | --- |
| Signature | `monita admin create --email <email>`. The password is read from a prompt, or from stdin when not a TTY. |
| Behavior | Validates the email and a password of 12 or more chars. Inserts `users(role=super_admin)`. (AC-01.01) |
| Output | Exit 0, prints `created super admin <email>`. |
| Errors | Exit 1 `email already exists` (AC-01.02). Exit 1 `password too short`. |

## 2.2 Sign-in and invites

The pages `/login` and `/invite/{token}` are React pages; they call these JSON routes (conventions 1.10).

| Method | Path | Access | Input | Behavior | AC |
| --- | --- | --- | --- | --- | --- |
| POST | `/api/ui/login` | public, 60/min per IP | `email`, `password` | Verify, create a session, set the cookie, 204. A wrong pair: 422 with one generic message and no field. | |
| POST | `/api/ui/logout` | session | | Delete the session, clear the cookie, 204. | |
| GET | `/api/ui/me` | session | | `email`, `role` (`super_admin` or `tl`), `is_admin`. 401 without a session. | |
| GET | `/api/ui/invites/{token}` | public | | 200 `{usable: true}` while the invite is unused and unexpired; 410 otherwise. | AC-03.01, AC-03.03, AC-02.02 |
| POST | `/api/ui/invites/{token}` | public, 60/min per IP | `password`, `password_confirm` | Check the invite first: unknown, used, expired or disabled user is 410 before any hashing. A mismatch is 422 on `password_confirm`; a short password is 422 on `password`. Then hash, mark used, delete older sessions, create a session, set the cookie, 204. | AC-03.02, AC-04.01 |

## 2.3 Admin pages (server-rendered until R6)

| Method | Path | Access | Input | Behavior | AC |
| --- | --- | --- | --- | --- | --- |
| GET | `/admin/users` | super_admin | | List TLs with status | |
| POST | `/admin/users` | super_admin | `email` | Create TL, send invite | AC-02.01 |
| POST | `/admin/users/{id}/invite` | super_admin | | New invite for an active TL; any other target is 404. Older invites and the user's sessions are deleted. | AC-04.01, AC-04.02 |
| POST | `/admin/users/{id}/disable` | super_admin | | Active TL only; any other target is 404. Set `disabled_at`, delete sessions and invites. Disabling is final in v1. | |
