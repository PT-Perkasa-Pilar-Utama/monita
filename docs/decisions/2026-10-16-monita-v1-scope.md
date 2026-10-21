# Monita v1 scope: decisions

Date: 2026-10-16

Monita is a central uptime and incident service for every project in the IT
department. It is a small version of Better Stack. The main persona is the tech
lead (TL) of each project.

Motto: easy to integrate, simple features, lightweight, fast, reliable, small
footprint.

## Terms

| Term | Meaning |
|------|---------|
| Monitor | Monita sends a request to a project URL on an interval. |
| Heartbeat | A project job sends a request to a Monita ping URL. A missing ping is a failure. |
| Incident | A period of failure of one monitor or heartbeat, or a period the TL opens by hand. |
| Status update | A message from the TL on an incident, with a label. |
| Ref ID | An ID that the project backend gets from Monita for one end user or for one ticket. |
| Ticket | A bug report that a project user submits with a ref ID. |

| # | Question | Decision | Why |
|---|----------|----------|-----|
| Q1 | Who owns a project? | The super admin creates TL accounts. A TL creates and owns projects. One owner per project. The super admin can reassign a project. | Simplest model that survives a TL who leaves. |
| Q2 | How does an alert reach people? | Each project has an alert setup: a list of email addresses and one optional webhook URL. Alerts fire on incident open and on incident close. | A webhook covers Slack, Teams, Telegram and others without one integration each. |
| Q3 | How are alerts categorized? | Severity, set by the TL on each monitor and heartbeat: critical or warning. Each level has its own emails and webhook. Default: critical goes to email and webhook, warning goes to email only (superseded by v1.1 Q27: both start with email only). Every alert also states its type: down, heartbeat missed, recovered. | The TL knows what matters. Monita knows what happened. |
| Q4 | Who closes an incident? | Monita opens an incident after 2 failed checks in a row. Monita closes it when the check passes. The TL can post status updates at any time, also after close. The TL can open an incident by hand. | Downtime figures stay exact. No incident stays open by mistake. |
| Q5 | Which labels can a status update have? | Investigating, Identified, Monitoring, Resolved. Resolved closes a manual incident. Monita sets Resolved on an automatic incident. | Atlassian Statuspage set. Visitors know it. |
| Q6 | What can a monitor check? | HTTP(S) only. URL, interval (30s, 1m, 3m, 5m; default 1m), timeout (default 10s), expected status (default 2xx), optional body keyword. | HTTP covers almost every project. Other types cost code and tests. |
| Q7 | How does a heartbeat work? | Secret URL `/ping/<token>`. GET, HEAD or POST counts as a ping. The TL sets a period and a grace time. No ping in period + grace opens an incident. The next ping closes it. `/ping/<token>/fail` reports a failure at once. | Same model as Better Stack and Healthchecks. |
| Q8 | What does the public status page show? | One page per project at `/status/<project-slug>`. It shows monitors and heartbeats that the TL marks public (default: hidden), a 90-day uptime bar for each, and incidents with their status updates. A public read-only JSON API without a key gives the same data. | Opt-in visibility keeps internal names private. The API lets a project build its own page. |
| Q9 | How does a project user get a ref ID and submit a ticket? | The project backend calls `POST /api/v1/ref-ids` with the project API key. `single_use: false` gives a reporter ID to keep on the user record. `single_use: true` gives an ID for one ticket. The user submits a ticket through the Monita form or the project UI through the API. A ticket has a title, a description and an optional page URL. No uploads. Limits: 5 tickets per ref ID per day, 100 tickets per project per day. The TL can revoke a ref ID. Optional IP allowlist per project, off by default. | The API key proves the caller. An `Origin` check does not, because a script can fake the header. The project decides who gets an ID, so the project owns spam control. |
| Q10 | Does Monita limit traffic? | Yes, in both directions. Inbound: 60 requests/min per IP on public pages and the ticket form, 120/min per API key, 1 ping/s per heartbeat (extra pings get 429 and still count as alive). Outbound: each monitor gets a random start offset in its interval, and at most 50 checks run at once. The super admin can change all limits. | One broken caller or one burst of checks must not take Monita down. |
| Q11 | What columns does the kanban have, and what is public? | Triage, Backlog, In progress, Done. Done needs a close reason: Fixed, Won't fix or Duplicate. Triage is private. A ticket is public after the TL moves it out of Triage. Public fields: title, column, created date, last update. The description stays private. The TL can rewrite the public title. A reporter can see their own tickets by ref ID. | Bug reports hold personal data. Spam never reaches the public board. |
| Q12 | Where does data live, and for how long? | Turso (`turso` crate), used like plain SQLite. Rules: standard SQLite features only (no BEGIN CONCURRENT, MVCC or Postgres frontend); nightly backup to a different disk. Raw check results: 7 days. Daily summaries: forever. Incidents, status updates, tickets: forever. | Turso uses the SQLite file format, so `rusqlite` can open the same file if Turso fails. Turso is pre-1.0 (v0.8.3). Supabase announced it would acquire Turso on 2026-10-02. |
| Q13 | How do people sign in? | Email and password only. The first super admin comes from `monita admin create`. The super admin creates a TL, and Monita emails an invite link. A forgotten password gets a new invite link from the super admin. The TL creates, names and revokes project API keys. | No SSO setup. No self-service reset to abuse. |
| Q14 | Does the TL hear about new tickets? | One email digest per project per day, listing new tickets in Triage. The warning-level webhook fires for each new ticket, if it is set. | A burst of tickets gives one email. |
| Q15 | Who watches Monita? | Monita exposes `/healthz`. An outside service watches it, for example a free Better Stack or Healthchecks heartbeat that Monita pings every minute. v1 documents this setup and does not build it. | Monita cannot send an alert about its own death. |

## Deferred

| Branch | Reason | Who decides |
|--------|--------|-------------|
| Monitor types other than HTTP (TCP, ping, DNS) | Not needed in v1 | The user, on request from a TL |
| SSO, self-service password reset, 2FA | Not needed in v1 | The user |
| File uploads on tickets | Storage cost and privacy risk | The user |

## Amendments

Added after v1 scope. A row amends a decision above; the original row stays as it was decided.

| # | Date | Amends | Decision | Why |
|---|------|--------|----------|-----|
| A1 | 2026-10-18 | Q9 | The team files tickets from the board. Such a ticket has `created_by` and no ref ID, no daily caps and no webhook. Q9 let only project users file. | Caps and the webhook guard against reporter traffic, and the team already knows its own tickets. |
| A2 | 2026-10-18 | Q14 | Ticket alerts are apart from the severities. The owner digest is a switch (on by default). The ticket webhook is a switch (off by default) with its own URL, and the warning webhook no longer hears tickets. Migration 0007. | Before, every new ticket called the warning webhook. |
| A3 | 2026-10-19 | Q14 | The ticket webhook can also hear each column move (`ticket.moved`). Off by default. Migration 0011. | Not recorded. |
| A4 | 2026-10-18 | Q2, Q14 | Each alert setting (Critical, Warning, Ticket alerts) has an on and off switch. Off keeps the recipients and the webhook. Migration 0008. | Off keeps the recipients and the webhook, so turning it back on restores them. |
| A5 | 2026-10-18 | Q5 | A TL may post Resolved on an open automatic incident. It closes, the resolved alert says "marked resolved by a tech lead", and the source starts over. A heartbeat gets a new deadline. Q5 said Monita sets Resolved on an automatic incident. | A job can be retired on purpose, and then no passing check ever arrives. |
| A6 | 2026-10-18 | Q2, Q4 | Each monitor and heartbeat has an alert switch and an incident switch (migration 0009). With the incident switch off, a failure sends a Down alert and recovery sends an Up alert, and no incident opens. With the alert switch off, nobody is told. | Not recorded. |
| A7 | 2026-10-19 | Q4 | The 2-failure rule is a default. Each monitor sets 1 to 99 failed checks in a row (migrations 0014 and 0018). | It was 2 for every monitor, so 2 stays the default and no monitor changes. |
| A8 | 2026-10-18 | Q1, Q13 | The super admin can promote, demote, enable, disable and invite any user, never their own account. Invites carry a role. | At least one active super admin always remains, because nobody changes their own account. |
| A9 | 2026-10-19 | Q1 | A third role, supervisor. It sees every project and the admin pages and changes nothing. A project owner cannot become one. Migration 0022. | Not recorded. |
| A10 | 2026-10-18 | Q1 | Only a super admin deletes a project, with everything in it. | Not recorded. |
| A11 | 2026-10-19 | Q6 | A monitor may send GET, HEAD, POST or PUT, with headers and a body. It may sign in with a bearer token, a user name and password, or a key in a header, each from a sealed secret in the project (migrations 0013 and 0015). Q6 said URL, interval, timeout, status and keyword. | A check must send the real values, so they are stored as entered. |
| A12 | 2026-10-19 | Q8 | The status page has tabs a project switches: Uptime (always), Incidents, Known issues, Report a bug, Changelog. A tab that is off answers 404. Q8 described one page. Migrations 0010, 0016, 0019 and 0021. | Each project chooses which public tabs visitors see. |
| A13 | 2026-10-19 | Q1 | Every new project gives a homepage URL, linked from its status page. Migration 0020. | The status page links to it. |
| A14 | 2026-10-19 | Q2 | A super admin can save the mail server on the admin Email page. `MONITA_SMTP_URL` is the fallback. | Not recorded. |
| A15 | 2026-10-19 | Q12 | Pages show times in a fixed UTC offset that a super admin sets. Storage stays UTC. There is no daylight saving. Daily summaries stay UTC days. | Not recorded. |
