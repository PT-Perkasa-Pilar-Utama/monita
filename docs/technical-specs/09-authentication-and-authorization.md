# 9. Authentication and Authorization

## 9.1 Mechanisms

| Caller | Mechanism |
| --- | --- |
| Super admin, TL, supervisor | Email and password. Server-side session in `sessions`, token in `monita_session` cookie. See SEC-01, SEC-02. The role is read on every request, so a role change holds from the next click. |
| Project backend | `Authorization: Bearer mk_...` project API key. See SEC-07, SEC-08. |
| Project job | The heartbeat token in the URL path. |
| Reporter | The ref ID in the form or JSON body. |
| Visitor | None. |

## 9.2 Flows

```text
first install:  monita admin create --email a@x  -> prompt password -> users(role=super_admin)
invite:         super admin -> invite(email, role) -> email link /invite/<token>
accept:         page /invite/<token> -> GET /api/ui/invites/<token> -> POST password -> hash, mark used, create session
sign-in:        POST /api/ui/login -> argon2 verify -> create session -> 204 with the cookie; the page goes to ?next= on this site, or /
two-factor:     POST /api/ui/login -> argon2 verify -> challenge (hash stored, 5 min, 5 tries) -> 200 {challenge}
                POST /api/ui/login/code -> time code (later step than the last) or recovery code -> session -> 204
sign-out:       POST /api/ui/logout -> delete session row
reset:          super admin -> new invite -> old invites and sessions deleted, two-factor off
```

## 9.3 Role matrix

Roles: `super_admin`, `tl` and `supervisor`. A supervisor sees every project and the admin pages, and changes nothing. `CurrentUser` answers 403 to a supervisor on any method but `GET` and `HEAD` (SEC-22). The one exception is the user's own account: two-factor sign-in and User settings (password, picture). Those routes take `AccountUser`, which lets a supervisor write.

| Action | Super admin | Supervisor | TL (owner) | TL (not owner) | API key | Public |
| --- | --- | --- | --- | --- | --- | --- |
| Invite with a role, change a role, disable, enable, send a new invite | yes (never your own account) | no | no | no | no | no |
| Reassign project, delete project | yes | no | no | no | no | no |
| Change settings (limits, time zone, mail server, Telegram bot, locations) | yes | no | no | no | no | no |
| View users and the admin pages | yes | yes | no | no | no | no |
| Turn another user's two-factor off | yes (never your own account) | no | no | no | no | no |
| Turn your own two-factor on or off | yes | yes | yes | yes | no | no |
| Change your own password or picture (User settings) | yes | yes | yes | yes | no | no |
| Create project | yes | no | yes | yes | no | no |
| View a project and its pages | yes (all) | yes (all) | yes | no (404) | `read` or `write`: its own project's detail, monitors, heartbeats, incidents and maintenance | no |
| Manage monitors and heartbeats (pause too), plan or end maintenance | yes | no | yes | no | `write` (own project) | no |
| Manage alert cards, ticket cards, keys, allowlist, secrets, status page tabs | yes | no | yes | no | no | no |
| Post status update, open manual incident, resolve an automatic incident by hand | yes | no | yes | no | `write` (own project) | no |
| Move tickets, rewrite public title, revoke ref ID, file a ticket from the board | yes | no | yes | no | no | no |
| Issue ref ID from the ticket board | yes | no | yes | no | no | no |
| Issue ref ID with `POST /api/v1/ref-ids` | no | no | no | no | `ref_ids` or `write` (own project) | no |
| Write a changelog release | yes | no | yes | no | no | no |
| Submit ticket | no | no | no | no | yes | with a ref ID |
| Ping heartbeat | no | no | no | no | no | with the token |
| View status page, public ticket board, public JSON | yes | yes | yes | yes | yes | yes |

A TL who is not the owner gets 404 on another project's admin pages, so project names do not leak.

An API key has a scope (v1.1 Q44): `ref_ids`, `read` or `write`. A `read` or `write` key works only under `/api/v1/projects/{slug}/`. `CurrentUser` turns it into a principal bound to the key's project, and a session cookie counts for nothing there. A slug of another project is 404 before the route runs. A `read` key is read-only as a supervisor is, and `CurrentUser` answers 403 to its writes. Routes list: api-specs 03 section 3.4.

What a supervisor or a `read` key does not see: a heartbeat's ping URL, monitor header values, the request body, the sign-in user name, a monitor URL's query (Q23), and the alert card and ticket card webhook URLs (SEC-22). A sealed secret's value is never shown to anyone (SEC-23).

Rules for roles:

- Nobody changes, disables or resets their own account on the Users page, so one active super admin always remains. `enable` takes no actor.
- A project owner cannot become a supervisor. Give the projects to another owner first.
- The owner list offers the acting super admin and the active TLs. A supervisor is never an owner.
