# 2. Authentication

Session strategy: [../technical-specs/07-security.md](../technical-specs/07-security.md) SEC-01 to SEC-03, SEC-06, SEC-11.

## 2.1 CLI: `monita admin create`

| Part | Contract |
| --- | --- |
| Signature | `monita admin create --email <email>`. The password is read from a prompt, or from stdin when not a TTY. |
| Behavior | Validates the email and a password of 12 or more chars. Inserts `users(role=super_admin)`. (AC-01.01) |
| Output | Exit 0, prints `created super admin <email>`. |
| Errors | Exit 1 `email already exists` (AC-01.02). Exit 1 `password too short`. |

## 2.2 HTML routes

| Method | Path | Access | Input | Behavior | AC |
| --- | --- | --- | --- | --- | --- |
| GET | `/login` | public | | Sign-in form | |
| POST | `/login` | public, 60/min per IP | `email`, `password` | Verify, create session, set cookie, 303 to `/`. Wrong pair: 422 with one generic message. | |
| POST | `/logout` | session | | Delete session, clear cookie, 303 to `/login` | |
| GET | `/invite/{token}` | public | | Password form if the invite is unused and unexpired. Otherwise an error page with 410. | AC-03.01, AC-03.03, AC-02.02 |
| POST | `/invite/{token}` | public | `password`, `password_confirm` | Hash, mark used, delete older sessions, create session, 303 to `/` | AC-03.02, AC-04.01 |
| GET | `/admin/users` | super_admin | | List TLs with status | |
| POST | `/admin/users` | super_admin | `email` | Create TL, send invite | AC-02.01 |
| POST | `/admin/users/{id}/invite` | super_admin | | New invite. Older invites and the user's sessions are deleted. | AC-04.01, AC-04.02 |
| POST | `/admin/users/{id}/disable` | super_admin | | Set `disabled_at`, delete sessions | |
