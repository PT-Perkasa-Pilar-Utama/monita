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

Roles: `super_admin`, `tl` and `supervisor`. A supervisor sees every project and the admin pages, and changes nothing. `CurrentUser` answers 403 to a supervisor on any method but `GET` and `HEAD` (SEC-22). The one exception is the user's own two-factor sign-in: those routes take `AccountUser`, which lets a supervisor write.

| Action | Super admin | Supervisor | TL (owner) | TL (not owner) | API key | Public |
| --- | --- | --- | --- | --- | --- | --- |
| Invite with a role, change a role, disable, enable, send a new invite | yes (never your own account) | no | no | no | no | no |
| Reassign project, delete project | yes | no | no | no | no | no |
| Change settings (limits, time zone, mail server) | yes | no | no | no | no | no |
| View users and the admin pages | yes | yes | no | no | no | no |
| Turn another user's two-factor off | yes (never your own account) | no | no | no | no | no |
| Turn your own two-factor on or off | yes | yes | yes | yes | no | no |
| Create project | yes | no | yes | yes | no | no |
| View a project and its pages | yes (all) | yes (all) | yes | no (404) | no | no |
| Manage monitors, heartbeats, alert routes, ticket alerts, keys, allowlist, secrets, status page tabs | yes | no | yes | no | no | no |
| Post status update, open manual incident, resolve an automatic incident by hand | yes | no | yes | no | no | no |
| Move tickets, rewrite public title, revoke ref ID, file a ticket from the board | yes | no | yes | no | no | no |
| Issue ref ID from the ticket board | yes | no | yes | no | no | no |
| Issue ref ID with `POST /api/v1/ref-ids` | no | no | no | no | yes (own project) | no |
| Write a changelog release | yes | no | yes | no | no | no |
| Submit ticket | no | no | no | no | yes | with a ref ID |
| Ping heartbeat | no | no | no | no | no | with the token |
| View status page, public ticket board, public JSON | yes | yes | yes | yes | yes | yes |

A TL who is not the owner gets 404 on another project's admin pages, so project names do not leak.

What a supervisor does not see: a heartbeat's ping URL, monitor header values, the request body, the sign-in user name, and the alert and ticket webhook URLs (SEC-22). A sealed secret's value is never shown to anyone (SEC-23).

Rules for roles:

- Nobody changes, disables or resets their own account on the Users page, so one active super admin always remains. `enable` takes no actor.
- A project owner cannot become a supervisor. Give the projects to another owner first.
- The owner list offers the acting super admin and the active TLs. A supervisor is never an owner.
