# 7. Security

## 7.1 Threat model

| Boundary | Threat | Control |
| --- | --- | --- |
| Browser to Monita (admin UI) | Session theft | SEC-01, SEC-02 |
| Browser to Monita (admin UI) | Cross-site request forgery | SEC-03 |
| Browser to Monita (admin UI) | Stored XSS through names, ticket text, status messages | SEC-04, SEC-05 |
| Browser to Monita (sign-in) | Password guessing | SEC-06, SEC-12 |
| Project backend to Monita API | Stolen or guessed API key | SEC-07, SEC-08 |
| Public to `/ping`, `/status`, ticket form | Flooding, spam | SEC-12, SEC-13 |
| Public to ticket data | Personal data leak | SEC-14 |
| Monita to targets and webhooks | Request to an internal address | Accepted by design. See SEC-15. |
| Monita to SMTP | Credential leak | SEC-16 |
| Admin surface | Reset endpoint in production | SEC-17 |
| Database file | Theft from disk or backup | SEC-18 |

## 7.2 Controls

| ID | Control | Implementation |
| --- | --- | --- |
| SEC-01 | Session cookie | `monita_session`: 32 random bytes, `HttpOnly`, `Secure` (unless `MONITA_ENV=dev`), `SameSite=Lax`, `Path=/`. Only the SHA-256 hash is stored. |
| SEC-02 | Session revocation | A new invite, a disabled user, or sign-out deletes the session rows. Sessions expire after 14 days. |
| SEC-03 | CSRF | Every non-GET request with a session must carry an `Origin` header that matches `MONITA_BASE_URL`. Otherwise 403. |
| SEC-04 | Output escaping | askama escapes HTML by default. `|safe` is forbidden outside `templates/ui/` icons. |
| SEC-05 | Content Security Policy | `default-src 'self'; img-src 'self' data:; style-src 'self'; script-src 'self'; frame-ancestors 'none'`. Plus `X-Content-Type-Options: nosniff` and `Referrer-Policy: same-origin`. |
| SEC-06 | Password storage | Argon2id with the crate defaults. Minimum length 12. |
| SEC-07 | API key storage | Keys are `mk_` + 26 base32 chars (130 bits). Only the hash is stored. Shown once. |
| SEC-08 | IP allowlist | Optional per project. Checked after key lookup. The client IP comes from the socket, or from `X-Forwarded-For` only when `MONITA_TRUST_PROXY=true`. |
| SEC-09 | Token entropy | Invite, ref ID and heartbeat tokens: 128 bits or more from `rand::rngs::OsRng`. |
| SEC-10 | Constant-time compare | Token lookups go through the hash index, so no secret is compared byte by byte in code. |
| SEC-11 | Invite expiry | 72 h, single use. |
| SEC-12 | Rate limits | Fixed window, in memory: 60/min per IP on public pages, the ticket form and `/login`; 120/min per API key; 1/s per heartbeat. Over the limit returns 429 with `Retry-After`. |
| SEC-13 | Ticket caps | 5 per ref ID per day, 100 per project per day. |
| SEC-14 | Ticket privacy | Triage is never public. Public views show only the title (or the public title), stage and dates. The description and the reporter are never public. |
| SEC-15 | Outbound targets | Monitors and webhooks may target internal addresses. Only the owner of a project or a super admin can set them. Probe bodies are capped at 1 MB. Redirects are followed up to 5 times. |
| SEC-16 | Secrets in config | SMTP credentials come from the environment file (mode 0600, owned by the service user). They are never logged. |
| SEC-17 | Reset endpoint | Registered only when `MONITA_ENV` is `dev` or `test`. Enforced at route registration, not by authorization. In prod the path returns 404. |
| SEC-18 | Data at rest | The DB file and backups have mode 0600 and belong to the `monita` user. |
| SEC-19 | Request limits | Body limit 64 KB on forms and JSON. Ticket description at most 10 000 chars. |
| SEC-20 | Logs | No passwords, tokens, keys or ticket descriptions in logs. Tokens appear as their first 6 chars at most. |

## 7.3 Operational endpoints

| Endpoint | Exposure |
| --- | --- |
| `GET /healthz` | Public. The body holds status flags only. |
| `POST /admin/reset-state` | Absent in prod (SEC-17). |

## 7.4 CORS

The public JSON (`/api/v1/public/*`) sends `Access-Control-Allow-Origin: *`, so a project can read it from the browser. All other routes send no CORS headers.
