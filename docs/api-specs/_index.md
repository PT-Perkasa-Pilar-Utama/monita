# Monita v1: API Specification

**Version:** 1.0  
**Date:** 2026-10-16  
**Author:** snowfluke  
**Status:** Approved for build  
**Phase:** 2 Design  

Base address: `{MONITA_BASE_URL}`. JSON API under `/api/v1`. Protocol: REST, per [../technical-specs/04-tech-stack.md](../technical-specs/04-tech-stack.md).

## Operation Status Tracker

Legend: `OK` implemented and tested, `WIP` in progress, `TODO` not started, `SCAFFOLD` stub returning a mock.

### JSON API

| Operation | File | Status |
| --- | --- | --- |
| `POST /api/v1/ref-ids` | [08](08-tickets.md#81-post-apiv1ref-ids) | OK |
| `POST /api/v1/tickets` | [08](08-tickets.md#82-post-apiv1tickets) | OK |
| `GET /api/v1/tickets/mine` | [08](08-tickets.md#83-get-apiv1ticketsmine) | OK |
| `GET /api/v1/public/status/{slug}` | [07](07-status.md#71-get-apiv1publicstatusslug) | OK |
| `GET, HEAD, POST /ping/{token}` | [04](04-checks.md#41-get-head-post-pingtoken) | OK |
| `GET, HEAD, POST /ping/{token}/fail` | [04](04-checks.md#42-get-head-post-pingtokenfail) | OK |
| `GET /healthz` | [09](09-system.md#91-get-healthz) | OK |
| `POST /admin/reset-state` | [09](09-system.md#92-post-adminreset-state) | OK (header token in dev and test; the session option is not built) |

### HTML routes

| Group | File | Status |
| --- | --- | --- |
| Sign-in, invites, TL management | [02](02-authentication.md) | OK |
| Projects, API keys | [03](03-projects.md) | OK |
| Monitors, heartbeats | [04](04-checks.md#43-html-routes) | OK |
| Incidents | [05](05-incidents.md) | OK |
| Alert routes | [06](06-notify.md) | OK |
| Status page | [07](07-status.md#72-html-routes) | OK |
| Tickets kanban, public board, ticket form | [08](08-tickets.md#85-html-routes) | OK |

## Files in This Directory

| File | Content |
| --- | --- |
| [01-conventions.md](01-conventions.md) | Envelope, errors, status codes, rate limits, roles |
| [02-authentication.md](02-authentication.md) | Sessions, sign-in, invites, TL management |
| [03-projects.md](03-projects.md) | Projects, API keys, allowlist |
| [04-checks.md](04-checks.md) | Ping API, monitor and heartbeat routes |
| [05-incidents.md](05-incidents.md) | Incident routes |
| [06-notify.md](06-notify.md) | Alert route settings, webhook payload |
| [07-status.md](07-status.md) | Public status JSON and page |
| [08-tickets.md](08-tickets.md) | Ref IDs, tickets |
| [09-system.md](09-system.md) | Health, reset-state |

## Companion Documents

| Document | Path |
| --- | --- |
| Technical specs | [../technical-specs/](../technical-specs/) |
| Business docs | [../business/](../business/) |
