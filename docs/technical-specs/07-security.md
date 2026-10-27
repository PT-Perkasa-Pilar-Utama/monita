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
| Read-only role | A supervisor changes data or reads a credential | SEC-22 A monitor URL and a project homepage may not carry a user name or password (`https://user:pass@host`), so a supervisor never reads one there: the monitor's sign-in fields hold them instead. |
| Stored credentials | Secrets, header values and the mail password leave Monita in a response or a log | SEC-23, SEC-24, SEC-25 |
| Ref IDs | The team cannot tell ref IDs apart, or a stored hint weakens one | SEC-26 |

## 7.2 Controls

| ID | Control | Implementation |
| --- | --- | --- |
| SEC-01 | Session cookie | `monita_session`: 128 random bits (26 base32 chars), `HttpOnly`, `Secure` when `MONITA_BASE_URL` is `https://`, `SameSite=Lax`, `Path=/`. Only the SHA-256 hash is stored. |
| SEC-02 | Session revocation | A new invite, a disabled user, or sign-out deletes the session rows. Sessions expire after 14 days. |
| SEC-03 | CSRF | Every non-GET request outside `/api/v1/`, `/ping/` and reset-state must carry an `Origin` header equal to `MONITA_BASE_URL`. Otherwise 403. When `MONITA_BASE_URL` is a loopback address (`localhost`, `127.0.0.1`, `[::1]`), any loopback name with the same scheme and port also passes, so `http://127.0.0.1:7466` works beside the default `http://localhost:7466`. Only a page on the same machine and port can send such an origin. This covers the frontend's session-cookie calls under `/api/ui/`. |
| SEC-04 | Output escaping | React escapes text by default. `dangerouslySetInnerHTML` is forbidden. The server sends JSON only, never HTML built from data. |
| SEC-05 | Content Security Policy | `default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; script-src 'self'; font-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'self'`. Inline styles are allowed because Radix places popovers and dialogs with a `style` attribute; scripts stay limited to Monita's own files. Plus `X-Content-Type-Options: nosniff` and `Referrer-Policy: same-origin`. |
| SEC-06 | Password storage | Argon2id with the crate defaults. Minimum length 12. |
| SEC-07 | API key storage | Keys are `mk_` + 26 base32 chars (128 random bits). Only the hash is stored. Shown once. |
| SEC-08 | IP allowlist | Optional per project. Checked after key lookup. The client IP comes from the socket, or from the right-most `X-Forwarded-For` entry only when `MONITA_TRUST_PROXY=true`. The proxy appends that entry; the entries left of it are client-supplied. |
| SEC-09 | Token entropy | Session, invite, API key, ref ID and heartbeat tokens: 128 bits from the OS (`getrandom`). |
| SEC-10 | Constant-time compare | Token lookups go through the hash index, so no secret is compared byte by byte in code. |
| SEC-11 | Invite expiry | 72 h, single use. |
| SEC-12 | Rate limits | Fixed window, in memory, and never per caller address (v1.1 Q45): an office, a school or a mobile carrier puts many people behind one address, and a limit there locks them all out. `POST /api/ui/login` takes 10 wrong passwords per account per 15 minutes (`signin:{email}`, an unknown email too, so the answer never shows whether an account exists); two-factor codes and password changes have their own per-account budgets; 120/min per API key; 1/s per heartbeat; 300/min per location token (`probe:{id}`). Wrong API keys, unknown ping tokens, location tokens and invite links are not limited: each is random, so guessing one is hopeless. Ticket filing is capped per ref ID and per project each day. Over the limit returns 429 with `Retry-After`. |
| SEC-13 | Ticket caps | 5 per ref ID per day, 100 per project per day. |
| SEC-14 | Ticket privacy | Triage is never public. Public views show only the title (or the public title), stage and dates. The description and the reporter are never public. |
| SEC-15 | Outbound targets | Monitors and webhooks may target internal addresses. Only the owner of a project or a super admin can set them. Probe bodies are capped at 1 MB. Redirects are followed up to 5 times. A check that sends its own headers or a header-kind secret follows only redirects that stay on the monitored server (same host and port, or http to https on the same host), and fails with the redirect otherwise, so those headers never reach another party (`checks/probe/redirect.rs`). |
| SEC-16 | Secrets in config | SMTP credentials come from the mail server a super admin saved (`settings.mail`, SEC-25) or, when none is saved, from the environment file (mode 0600, owned by the service user). They are never logged. |
| SEC-17 | Reset endpoint | Registered only when `MONITA_ENV` is `dev` or `test`. Enforced at route registration, not by authorization. In prod the path returns 404. |
| SEC-18 | Data at rest | The DB file, its WAL, its lock file and backups have mode 0600 and belong to the `monita` user. Monita sets the mode each time it opens the database, whatever the umask. |
| SEC-19 | Request limits | Request bodies are capped at 128 KiB (413): a ticket of 10 000 four-byte characters, percent-encoded, is about 120 KB. The ping routes are the exceptions. They read a body only after the token and the limits pass. They read up to 16 MiB or 10 s and keep only the first 1 KB of a `/fail` body. Ticket description at most 10 000 chars. |
| SEC-21 | Status page privacy | Public pages and JSON never show an update's author, and show Monita's own detection entry as `Detected.` without its cause, which can name internal hosts. Incidents and their item lists are cut to public items. |
| SEC-20 | Logs | No passwords, tokens, keys, sealed secret values, header values, request bodies, the mail password or ticket descriptions in logs. Tokens appear as their first 6 chars at most. Email bodies are not logged and are cleared from the outbox once delivered or failed. A check's error text names no header value, body or credential. |
| SEC-22 | Supervisor is read-only | A supervisor sees every project and the admin pages and changes nothing. `CurrentUser` answers 403 to any method but `GET` and `HEAD`, and every signed-in route that changes data takes it, so one check covers them all. The views also hide what lets a reader act as the team: monitor header values (`Name: (hidden)`), a monitor URL's query and fragment (`https://host/path?(hidden)`, Q23), the request body and the sign-in user name read `(hidden)`, a heartbeat shows no ping URL, and alert card and ticket card webhook URLs and webhook keys (a Telegram chat ID or a PagerDuty routing key) read `(hidden)`. Hiding is a second layer; the 403 is the first. An owner cannot become a supervisor, and no supervisor is offered as an owner. The web app hides the write controls (`Writable`, `useCanEdit`). |
| SEC-23 | Sealed secrets | A project secret is written once and never shown again. No response carries a value: lists show the name and `used_by`. `projects::secrets::value_of` is the only door to a value, and only `checks::Job::read_secret` calls it: just before a probe on this server, or when a location's probe claims the check (SEC-28). A secret of another project is refused (422 `auth_secret_id`). A secret that monitors use cannot be deleted. Caps: 20 per project, 4 KB a value. ponytail: the value is stored as entered in the database file, like the mail password. Upgrade path: encrypt it with aws-lc-rs AEAD (already built in) under a key file beside the database. |
| SEC-24 | Check request values | A monitor's header values and request body are stored as entered, because the check must send them. Only the project owner and a super admin set them (SEC-15). `Host` and `Content-Length` are refused, at most 20 headers go out, a value is at most 4 KB and the body at most 64 KB. A header may not repeat the one Authentication sends. The detail page shows the method and the header names, and the Details panel's curl command reads a secret from a shell variable. |
| SEC-25 | Mail password | The password of the saved mail server is stored as entered, because Monita must send it to the server. The view says `has_password` and never returns it. A blank password in the form keeps the saved one. `Config` and `MailSettings` write their own `Debug`, which prints `(set)` for the SMTP URL, the reset token and the mail password, so a `{:?}` in a log leaks none of them. |
| SEC-26 | Ref ID tail | `ref_ids.tail` stores the last 4 of the 26 random characters of the raw ref ID, so the team can tell ref IDs apart. With the hash, 110 bits stay unknown. Ref IDs made before migration 0017 have no tail. |
| SEC-27 | Two-factor sign-in | Optional per user (Q19): RFC 6238, HMAC-SHA-1, 6 digits, 30 s, one step of drift, and no code twice (`users.totp_step`). The secret is stored as entered, like SEC-23, because Monita must compute the code; `/admin/users` says only `two_factor`. Recovery codes and sign-in challenges are random tokens stored as SHA-256 (SEC-07); the raw ones go out once. A challenge lives 5 minutes and dies at the fifth wrong code; the code step shares the `login:` rate bucket. Because a right password makes a new challenge each time, each account also has a budget of 10 wrong codes in each fixed 15-minute window (`2fa:{user}`, at most 20 in any 15 minutes across a window edge), across challenges and the turn-off form, checked inside the single writer so requests sent at once cannot pass it together; past it every code is refused with 429. A new invite clears the password, the sessions and two-factor at once. A super admin turns another user's two-factor off; Reset sign-in, `monita admin reset-password` and `monita admin reset-sign-in` (any active user) turn it off too. ponytail: the secret is plain in the database file, as SEC-23. Upgrade path: the same AEAD key file. |

| SEC-28 | Location tokens | A location's token is `ml_` + 26 base32 chars (128 random bits). Only the SHA-256 hash is stored; the token shows once, when a super admin adds the location, and `token_prefix` keeps its first 4 characters. A location token holder receives every check of that location, with the secret value of each monitor that signs in (technical-specs 14.4), so it is as sensitive as those secrets. Only a super admin adds or deletes a location; deleting it kills the token. `monita probe` reads the token from `MONITA_PROBE_TOKEN` so it stays out of `ps`. A probe's result carries no time: the server stamps it on receipt. |
| SEC-29 | Compression | JSON goes out as gzip when the client asks for it and its request is not cross-site (`platform::compress`). BREACH guesses a secret from the size of compressed answers to requests that another site starts. Under `SameSite=Lax` a top-level navigation from another site still carries the session cookie. A `cross-site` or `same-site` request gets plain JSON. A request without `Sec-Fetch-Site` (an old browser) gets gzip only when it carries no cookie, so a script with a key and a public page still get it. The web bundle ships precompressed (`.br`, `.gz`) and holds no secret. |

## 7.3 Operational endpoints

| Endpoint | Exposure |
| --- | --- |
| `GET /healthz` | Public. The body holds status flags only. |
| `POST /admin/reset-state` | Absent in prod (SEC-17). |

## 7.4 CORS

The public JSON (`/api/v1/public/*`) sends `Access-Control-Allow-Origin: *`, so a project can read it from the browser. All other routes send no CORS headers.
