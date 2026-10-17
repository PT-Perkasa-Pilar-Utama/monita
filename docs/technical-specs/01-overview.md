# 1. Overview

Monita is a self-hosted uptime and incident service for every project in the IT department. It is a small version of Better Stack. It checks project URLs, receives heartbeat pings from project jobs, opens and closes incidents, sends alerts, publishes a status page per project, and collects bug tickets from project users. It ships as one binary with one SQLite-format database file.

> **Terminology.** The terms Monitor, Heartbeat, Incident, Status update, Ref ID and Ticket are defined in the [decision log](../decisions/2026-10-16-monita-v1-scope.md#terms). `docs/GLOSSARY.md` takes them over when `project-docs` runs.

> **Language policy.** UI copy, code identifiers and docs are in English.

## 1.1 Goals

| Goal | Measure |
| --- | --- |
| Easy to integrate | A project adds a monitor, a heartbeat or a ticket flow with one HTTP call or one URL. |
| Lightweight | One binary under 30 MB. RSS under 100 MB at 500 monitors. See [08](08-non-functional-requirements.md). |
| Reliable | No lost alerts after a crash (outbox table). Two Monitas watch each other. |
| Simple | Four menus for a TL: Monitors, Heartbeats, Incidents, Tickets. Settings for the super admin. |

## 1.2 Scope by module

### 1.2.1 accounts
- Create the first super admin from the CLI (US-01).
- Create TLs, invite links, password set and reset (US-02, US-03, US-04).

### 1.2.2 projects
- Projects, ownership and reassignment (US-05, US-06).
- API keys and the IP allowlist (US-07, US-08).

### 1.2.3 checks
- HTTP monitors and recent checks (US-09, US-10).
- The scheduler with offsets and the concurrency cap (US-11).
- Heartbeats, pings and fail pings (US-12, US-13, US-14).
- Daily summaries and the retention sweep (US-32).

### 1.2.4 incidents
- Automatic open and close (US-17, US-18).
- Status updates and manual incidents (US-19, US-20).

### 1.2.5 notify
- Alert routes per severity, alerts on open and close (US-15, US-16).
- The daily ticket digest and the ticket webhook (US-30).

### 1.2.6 status
- Public status page and public JSON (US-21, US-22).

### 1.2.7 tickets
- Ref IDs, ticket submission, ticket board, public board, reporter view (US-23 to US-29).

### 1.2.8 platform
- Rate limits (US-31), nightly backup (US-33), `/healthz` and the peer ping (US-34), settings, database and migrations.

### 1.2.9 Out of scope
- Monitor types other than HTTP(S).
- SSO, 2FA, self-service password reset.
- Escalation, on-call schedules, SMS, phone calls.
- File uploads on tickets.
- Remote probes, hot standby, more than one instance on one database. See [02 section 2.5](02-system-architecture.md#25-replication-model).

## 1.3 User base

| Persona | Count (assumed) |
| --- | --- |
| Super admin | 1 to 3 |
| TL | 10 to 50 |
| Projects | 10 to 100 |
| Visitors and reporters | Unbounded, public |

## 1.4 Business workflow

```text
SUPER ADMIN
  monita admin create                     -> first super admin
  create TL(email)                        -> invite email
  reassign project(project, TL)

TL
  accept invite -> set password -> sign in
  create project(name, slug)
  create API key -> give to the project backend
  set alert routes(critical, warning)
  create monitor(url, interval, severity, public)
  create heartbeat(period, grace, severity, public) -> give ping URL to the job
  on alert:
    open incident page
    post status update(Identified, "cause found")
    post status update(Monitoring, "fix deployed")
    Monita closes the incident when the check passes
  daily: read ticket digest -> move tickets out of Triage -> work -> Done(reason)

PROJECT BACKEND
  POST ref-ids(single_use?) -> store or hand to the user

PROJECT JOB
  on success: GET /ping/<token>
  on failure: GET /ping/<token>/fail

REPORTER
  submit ticket(ref ID, title, description, page URL)
  view own tickets(ref ID)

VISITOR
  /status/<slug>           status page
  /status/<slug>/tickets   public ticket board
```

## 1.5 Assumptions and known constraints

1. Two builders (the user and Claude) build v1 in 2 days. If time runs short, Sprint 4 (tickets) moves out.
2. Monita runs on a Linux VM in the company data center, behind the company reverse proxy or directly.
3. A company SMTP relay exists.
4. A second Monita runs at another site and watches the first through a heartbeat (peer watch).
5. Turso is pre-1.0. Monita uses standard SQLite features only, so `rusqlite` can open the same file. See [04](04-tech-stack.md).
6. All times are stored in UTC.
