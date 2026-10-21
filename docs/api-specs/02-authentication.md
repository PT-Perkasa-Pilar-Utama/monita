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
| POST | `/api/ui/login` | public, 60/min per IP (`login:` bucket) | `email`, `password` | Verify, create a session, set the cookie, 204. With two-factor on: no session yet; 200 `{two_factor: true, challenge}` and the page asks for the code (2.4). A wrong pair: 422 with one generic message and no field. | |
| POST | `/api/ui/logout` | session | | Delete the session, clear the cookie, 204. | |
| GET | `/api/ui/me` | session | | `email`, `role` (`super_admin`, `tl` or `supervisor`), `is_admin` (changes anything: a super admin), `sees_all` (sees every project and the admin pages: a super admin or a supervisor), `read_only` (a supervisor, 1.9) and `utc_offset_min` (minutes east of UTC; the page shows times in it, 4.4). 401 without a session. | |
| GET | `/api/ui/invites/{token}` | public | | 200 `{usable: true}` while the invite is unused and unexpired; 410 otherwise. | AC-03.01, AC-03.03, AC-02.02 |
| POST | `/api/ui/invites/{token}` | public, 60/min per IP | `password`, `password_confirm` | Check the invite first: unknown, used, expired or disabled user is 410 before any hashing. A mismatch is 422 on `password_confirm`; a short password is 422 on `password`. Then hash, mark used, delete older sessions, create a session, set the cookie, 204. | AC-03.02, AC-04.01 |

## 2.3 Admin users

The React page `/admin/users` calls these routes (conventions 1.10). A tech lead gets 403 on each. A supervisor may read (GET) and gets 403 on every write (1.9). No route acts on your own account: the target of a change is never you.

Roles: `super_admin`, `tl`, `supervisor`. A `supervisor` reads everything and changes nothing.

| Method | Path | Access | Input | Behavior | AC |
| --- | --- | --- | --- | --- | --- |
| GET | `/api/ui/admin/users` | super_admin, supervisor | | Every account (500 at most): `id`, `email`, `role`, `status` (`active`, `invited` before a password is set, `disabled`), `created_at` and `two_factor` (on or off). Never a password hash or a two-factor secret. | |
| POST | `/api/ui/admin/users/{id}/two-factor/off` | super_admin | | Turn off the two-factor sign-in of an active account that is not yours, for someone who lost the phone and the recovery codes: the secret, the recovery codes and open sign-in challenges are deleted, 204. A disabled or unknown target is 404; your own account is 422 (turn your own off on `/account` with a code). | #12 |
| POST | `/api/ui/admin/users` | super_admin | `email`, `role` (`tl`, `supervisor` or `super_admin`; blank is `tl`) | Create the account and queue the invite email. 201 `{email, link}`: the link is shown this once; only its hash is stored. A bad or taken email is 422 on `email`; another role value is 422 on `role`. | AC-02.01 |
| POST | `/api/ui/admin/users/{id}/invite` | super_admin | | New invite for any account that is not disabled and is not yours: `{email, link}`. It resets their sign-in. Older invites and the user's sessions are deleted, and two-factor is turned off. A disabled or unknown target is 404; your own account is 422. | AC-04.01, AC-04.02 |
| POST | `/api/ui/admin/users/{id}/disable` | super_admin | | Disable an enabled account that is not yours. Set `disabled_at`, delete sessions and invites, 204. Their projects stay theirs. A target already disabled or unknown is 404; your own account is 422. | |
| POST | `/api/ui/admin/users/{id}/enable` | super_admin | | Undo a disable, 204. A user who set a password signs in with it again; one who did not needs a new invite. A target that is not disabled, or unknown, is 404. | |
| PUT | `/api/ui/admin/users/{id}/role` | super_admin | `role` (`tl`, `supervisor` or `super_admin`) | Change the role of an enabled account that is not yours, 204. The role is read on every request, so it holds from the next click. An unknown value is 422 on `role`. A project owner cannot become a supervisor: 422 on `role`, "Give their projects to another owner first." A disabled or unknown target is 404; your own account is 422. | |

Two rules keep access safe. At least one active super admin always remains, because nobody changes their own account. A supervisor is never offered as a project owner (3.1, `owners`).

## 2.4 Two-factor sign-in

v1.1 decision Q19. Codes follow RFC 6238: HMAC-SHA-1, 6 digits, 30-second steps, one step of drift either way. A code works once: Monita stores the last step used and accepts only later ones. A recovery code is 10 letters and digits, shown as `abcde-fghij`; case, spaces and dashes do not matter. The page `/account` calls the `/api/ui/me/two-factor` routes. They take a session of any role, a supervisor's too: securing one's own account changes nothing others see.

| Method | Path | Access | Input | Behavior | AC |
| --- | --- | --- | --- | --- | --- |
| POST | `/api/ui/login/code` | public, the `login:` bucket of `/api/ui/login` | `challenge` (from `/api/ui/login`), `code` (a time code or a recovery code) | A right code uses the challenge up, creates a session, sets the cookie, 204. A wrong code is 422 on `code` and counts. The fifth wrong code, an unknown or used challenge, or one older than 5 minutes is 410: sign in again. A recovery code is used up. An account has 10 wrong codes in each fixed 15-minute window, counted across sign-in challenges and this page; past them every code, even a right one, is 429 `too_many_tries` until the window ends. Two windows meet at their edge, so at most 20 wrong codes fall within any 15 minutes. | #12 |
| GET | `/api/ui/me/two-factor` | session | | `on`, and `codes_left`: unused recovery codes. | #12 |
| POST | `/api/ui/me/two-factor` | session | | A new secret: `{secret, link, qr}`. `secret` is base32; `link` is the `otpauth://totp/Monita:<email>?secret=...&issuer=Monita` link apps read. `qr` is that link as an SVG document (black modules on white, 4 px each) for an `<img>`. Nothing changes until confirm; a second call replaces the waiting secret. Already on: 422. | #12 |
| POST | `/api/ui/me/two-factor/confirm` | session | `code` | A right code for the waiting secret turns two-factor on and answers `{recovery_codes}`: 10 codes, shown this once; only their SHA-256 is stored. A wrong code is 422 on `code`. No waiting secret, or already on: 422. Other sessions stay signed in. | #12 |
| POST | `/api/ui/me/two-factor/off` | session | `code` (a time code or a recovery code) | Turns it off and deletes the recovery codes, 204. A wrong code is 422 on `code`. Already off: 422. An account has 10 wrong codes in each fixed 15-minute window, counted across sign-in challenges and this page; past them every code, even a right one, is 429 `too_many_tries` until the window ends. Two windows meet at their edge, so at most 20 wrong codes fall within any 15 minutes. | #12 |

Two-factor also turns off when a super admin turns it off for the user (2.3), when a super admin sends the user a new invite (2.3), and for a super admin with `monita admin reset-password` (2.1).

