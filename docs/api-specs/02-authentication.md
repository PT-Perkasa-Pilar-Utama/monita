# 2. Authentication

Session strategy: [../technical-specs/07-security.md](../technical-specs/07-security.md) SEC-01 to SEC-03, SEC-06, SEC-11.

## 2.1 CLI: `monita admin create`

| Part | Contract |
| --- | --- |
| Signature | `monita admin create --email <email>`. The password is read from a prompt, or from stdin when not a TTY. |
| Behavior | Validates the email and a password of 12 or more chars. Inserts `users(role=super_admin)`. (AC-01.01) |
| Output | Exit 0, prints `created super admin <email>`. |
| Errors | Exit 1 `An account with this email already exists.` (AC-01.02). Exit 1 `Use at least 12 characters.` |

## 2.2 Sign-in and invites

The pages `/login` and `/invite/{token}` are React pages; they call these JSON routes (conventions 1.10).

| Method | Path | Access | Input | Behavior | AC |
| --- | --- | --- | --- | --- | --- |
| POST | `/api/ui/login` | public, 60/min per IP | `email`, `password` | Verify, create a session, set the cookie, 204. A wrong pair: 422 with one generic message and no field. | |
| POST | `/api/ui/logout` | session | | Delete the session, clear the cookie, 204. | |
| GET | `/api/ui/me` | session | | `email`, `role` (`super_admin`, `tl` or `supervisor`), `is_admin` (changes anything: a super admin), `sees_all` (sees every project and the admin pages: a super admin or a supervisor), `read_only` (a supervisor, 1.9) and `utc_offset_min` (minutes east of UTC; the page shows times in it, 4.4). 401 without a session. | |
| GET | `/api/ui/invites/{token}` | public | | 200 `{usable: true}` while the invite is unused and unexpired; 410 otherwise. | AC-03.01, AC-03.03, AC-02.02 |
| POST | `/api/ui/invites/{token}` | public, 60/min per IP | `password`, `password_confirm` | Check the invite first: unknown, used, expired or disabled user is 410 before any hashing. A mismatch is 422 on `password_confirm`; a short password is 422 on `password`. Then hash, mark used, delete older sessions, create a session, set the cookie, 204. | AC-03.02, AC-04.01 |

## 2.3 Admin users

The React page `/admin/users` calls these routes (conventions 1.10). A tech lead gets 403 on each. A supervisor may read (GET) and gets 403 on every write (1.9). No route acts on your own account: the target of a change is never you.

Roles: `super_admin`, `tl`, `supervisor`. A `supervisor` reads everything and changes nothing.

| Method | Path | Access | Input | Behavior | AC |
| --- | --- | --- | --- | --- | --- |
| GET | `/api/ui/admin/users` | super_admin, supervisor | | Every account (500 at most): `id`, `email`, `role`, `status` (`active`, `invited` before a password is set, `disabled`) and `created_at`. Never a password hash. | |
| POST | `/api/ui/admin/users` | super_admin | `email`, `role` (`tl`, `supervisor` or `super_admin`; blank is `tl`) | Create the account and queue the invite email. 201 `{email, link}`: the link is shown this once; only its hash is stored. A bad or taken email is 422 on `email`; another role value is 422 on `role`. | AC-02.01 |
| POST | `/api/ui/admin/users/{id}/invite` | super_admin | | New invite for any account that is not disabled and is not yours: `{email, link}`. It resets their sign-in. Older invites and the user's sessions are deleted. A disabled or unknown target is 404; your own account is 422. | AC-04.01, AC-04.02 |
| POST | `/api/ui/admin/users/{id}/disable` | super_admin | | Disable an enabled account that is not yours. Set `disabled_at`, delete sessions and invites, 204. Their projects stay theirs. A target already disabled or unknown is 404; your own account is 422. | |
| POST | `/api/ui/admin/users/{id}/enable` | super_admin | | Undo a disable, 204. A user who set a password signs in with it again; one who did not needs a new invite. A target that is not disabled, or unknown, is 404. | |
| PUT | `/api/ui/admin/users/{id}/role` | super_admin | `role` (`tl`, `supervisor` or `super_admin`) | Change the role of an enabled account that is not yours, 204. The role is read on every request, so it holds from the next click. An unknown value is 422 on `role`. A project owner cannot become a supervisor: 422 on `role`, "Give their projects to another owner first." A disabled or unknown target is 404; your own account is 422. | |

Two rules keep access safe. At least one active super admin always remains, because nobody changes their own account. A supervisor is never offered as a project owner (3.1, `owners`).
