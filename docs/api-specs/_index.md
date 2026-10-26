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
| `GET /api/v1/probe/work` | [04](04-checks.md#45-locations-v11-q26) | OK |
| `POST /api/v1/probe/results` | [04](04-checks.md#45-locations-v11-q26) | OK |
| `GET /healthz` | [09](09-system.md#91-get-healthz) | OK |
| `POST /admin/reset-state` | [09](09-system.md#92-post-adminreset-state) | OK (header token in dev and test; the session option is not built) |
| `monita setup`, `service`, `whereis`, `--env-file` | [10](10-cli.md) | OK |
| `monita probe` | [10](10-cli.md#105-monita-probe) | OK |
| `monita admin reset-sign-in` | [10](10-cli.md#106-monita-admin-reset-sign-in) | OK |
| Admin locations: list, add, delete | [04](04-checks.md#45-locations-v11-q26) | OK |

### Frontend routes (`/api/ui/`)

| Group | File | Status |
| --- | --- | --- |
| Sign-in, invites, `me` (`role`, `sees_all`, `read_only`) | [02](02-authentication.md#22-sign-in-and-invites) | OK |
| Admin users: list, create, invite, disable, enable, role | [02](02-authentication.md#23-admin-users) | OK |
| Admin users: turn two-factor off | [02](02-authentication.md#23-admin-users) | OK |
| Two-factor sign-in: code step, enroll (with `qr`), confirm, turn off | [02](02-authentication.md#24-two-factor-sign-in) | OK |
| User settings: password, picture | [02](02-authentication.md#25-user-settings) | OK |
| Projects: list (`owner`, `sort`, `state`, `pulse`), the list's chart (`/api/ui/projects-chart`), create, detail (`counts`), delete, overview, profile (`homepage_url`), icon, owners | [03](03-projects.md#31-frontend-routes) | OK |
| API keys, allowlist, public pages (`public-pages`) | [03](03-projects.md#31-frontend-routes) | OK |
| Secrets: list, create, replace, delete | [03](03-projects.md#33-secrets) | OK |
| Check options, monitors (method, headers, body, `fails_to_open`, `auth_*`), heartbeats (`last`, `alert`, `auto_incident`) | [04](04-checks.md#43-frontend-routes) | OK |
| Pause and resume a monitor or heartbeat (`PUT .../paused`, `paused_at`, state `paused`) | [04](04-checks.md#43a-pause-v11-q38) | OK |
| System page, mail server and its tests, limits, time zone | [04](04-checks.md#44-system-page) | OK |
| Project incidents, incident items, updates | [05](05-incidents.md#51-frontend-routes) | OK |
| Incidents across projects (`/api/ui/incidents`) | [05](05-incidents.md#52-incidents-across-projects) | OK |
| Maintenance windows | [05](05-incidents.md#53-maintenance-windows) | OK |
| Alert cards and ticket cards (list, add, edit, delete, switch, test), alerts preview | [06](06-notify.md#61-frontend-routes) | OK |
| The global Telegram bot: admin token, connect link and check | [06](06-notify.md#64-the-global-telegram-bot) | OK |
| Status page: status, history | [07](07-status.md#72-frontend-routes) | OK |
| Status page: past uptime, incidents by month | [07](07-status.md#72-frontend-routes) | OK |
| Ticket board, team-filed tickets, ref IDs from the board, public board, report form, reporter's tickets | [08](08-tickets.md#85-frontend-routes) | OK |
| Tickets across projects (`/api/ui/tickets`) | [08](08-tickets.md#86-tickets-across-projects) | OK |
| Changelog: team list and form | [11](11-changelog.md#111-team-routes) | OK |
| Changelog: public index and release | [11](11-changelog.md#112-public-routes) | OK |

## Files in This Directory

| File | Content |
| --- | --- |
| [01-conventions.md](01-conventions.md) | Envelope, errors, status codes, rate limits, roles |
| [02-authentication.md](02-authentication.md) | Sessions, sign-in, invites, user management and roles |
| [03-projects.md](03-projects.md) | Projects, API keys, allowlist, public pages, secrets |
| [04-checks.md](04-checks.md) | Ping API, monitor and heartbeat routes, the System page |
| [05-incidents.md](05-incidents.md) | Incident routes, the cross-project list, maintenance windows |
| [06-notify.md](06-notify.md) | Alert and ticket-alert settings, webhook payloads |
| [07-status.md](07-status.md) | Public status JSON and page routes |
| [08-tickets.md](08-tickets.md) | Ref IDs, tickets, the board, the cross-project list |
| [09-system.md](09-system.md) | Health, reset-state |
| [10-cli.md](10-cli.md) | The `monita` command line: setup, service, whereis, probe, reset-sign-in, the env file |
| [11-changelog.md](11-changelog.md) | Release notes per project: team routes and the public tab |

## Companion Documents

| Document | Path |
| --- | --- |
| Technical specs | [../technical-specs/](../technical-specs/) |
| Business docs | [../business/](../business/) |
