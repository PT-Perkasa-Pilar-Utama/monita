# 9. Authentication and Authorization

## 9.1 Mechanisms

| Caller | Mechanism |
| --- | --- |
| Super admin, TL | Email and password. Server-side session in `sessions`, token in `monita_session` cookie. See SEC-01, SEC-02. |
| Project backend | `Authorization: Bearer mk_...` project API key. See SEC-07, SEC-08. |
| Project job | The heartbeat token in the URL path. |
| Reporter | The ref ID in the form or JSON body. |
| Visitor | None. |

## 9.2 Flows

```text
first install:  monita admin create --email a@x  -> prompt password -> users(role=super_admin)
invite:         super admin -> invite_tl(email) -> email link /invite/<token>
accept:         GET /invite/<token> -> form -> POST password -> hash, mark used, create session
sign-in:        POST /login -> argon2 verify -> create session -> redirect /
sign-out:       POST /logout -> delete session row
reset:          super admin -> new invite -> old invites and sessions deleted
```

## 9.3 Role matrix

| Action | Super admin | TL (owner) | TL (not owner) | API key | Public |
| --- | --- | --- | --- | --- | --- |
| Create or disable TL, send invite | yes | no | no | no | no |
| Reassign project | yes | no | no | no | no |
| Change settings (limits, cap) | yes | no | no | no | no |
| Create project | yes | yes | yes | no | no |
| Manage monitors, heartbeats, alert routes, keys, allowlist | yes | yes | no | no | no |
| Post status update, open manual incident | yes | yes | no | no | no |
| Move tickets, rewrite public title, revoke ref ID | yes | yes | no | no | no |
| Issue ref ID | no | no | no | yes (own project) | no |
| Submit ticket | no | no | no | yes | with a ref ID |
| Ping heartbeat | no | no | no | no | with the token |
| View status page, public kanban, public JSON | yes | yes | yes | yes | yes |

A TL who is not the owner gets 404 on another project's admin pages, so project names do not leak.
