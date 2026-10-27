# Glossary

Monita's docs, pages and code use each of these words with one meaning. Say "tech lead", not "TL", and "ticket board", not "kanban". The scope decision log defines the first set in [Terms](decisions/2026-10-16-monita-v1-scope.md#terms).

| Term | Meaning |
| --- | --- |
| Project | One system your team runs. It has its own monitors, heartbeats, status page and tickets. |
| Monitor | A URL, a TCP port, a DNS name or a host that Monita checks on a fixed interval. |
| Check | One request that a monitor makes. It passes or fails. |
| Ping check | A monitor kind that sends one ICMP echo request to a host. Not the same as a heartbeat ping. |
| Location | Another machine that runs checks for this Monita with `monita probe` and reports the results. |
| Place | This server or a location, as a monitor checks from it. |
| Quorum | How many places must fail for a check to count as failed. |
| Heartbeat | A URL that a cron job calls when it runs. Monita expects the call before a deadline. |
| Ping | One call from a cron job to its heartbeat URL. Not the same as a ping check. |
| Incident | A record that something is down. Monita opens an automatic incident itself. A tech lead or a super admin opens a manual incident by hand. |
| Paused | A monitor or heartbeat a tech lead stopped for a while. Nothing checks it, nothing opens and no alert goes out until it resumes. Not a maintenance window. |
| Held | A missed heartbeat or a failed ping that waits for a maintenance window to end. The incident opens when the window ends, unless a good ping arrives first. |
| Maintenance window | A planned time when a project's chosen monitors and heartbeats open no incident and send no alert. The status page shows them as Maintenance. |
| Planned work | A class a tech lead gives a closed incident when the downtime was planned. It leaves availability, charts and the status page. The team still sees it. |
| False alarm | A class a tech lead gives an incident that was not a real outage. It is treated like planned work. Count it again undoes either class. |
| Pin | A mark a user puts on a project, monitor or heartbeat so it comes first in their lists. Each user pins at most 6 of each kind. |
| API key scope | What a project API key may do: `ref_ids` issues ref IDs only, `read` reads the project and `write` also changes it. |
| Two-factor sign-in | A 6-digit code from an authenticator app that Monita asks for after the password, when the user turned it on. |
| Recovery code | One of ten codes shown once when two-factor sign-in is turned on. Each signs in once without the phone. |
| Alert | An email or a webhook that Monita sends. It tells the team that an incident opened or closed, that a source went down or up, that a certificate will expire, or that a ticket was filed or moved. |
| Alert level | Critical or warning, set on each monitor, heartbeat and incident. Each alert card listens to one level or to both. It is a setting, not a state. The JSON field is `severity`. |
| Alert card | One place that receives incident alerts: an email list or a webhook. A project has two fixed cards that email its owner and up to ten of its own. Each card has a switch and a test button. |
| Ticket card | An alert card that hears tickets instead of incidents: the daily digest, new tickets or ticket moves. A card hears incidents or tickets, never both. |
| Global Telegram bot | The one Telegram bot that a super admin sets up for the whole Monita. A project connects a chat to it with a link. |
| Webhook | A message that Monita posts to a URL, for example a Slack or Google Chat channel. |
| Status page | The public page of a project. Anyone with the link can read it. |
| Ticket | A bug report from a user. It moves through triage, backlog, in progress and done. |
| Ticket board | The tech lead's view of every ticket in a project. The public board on the status page leaves out tickets in triage. |
| Ref ID | A code that a project's backend gives one of its users. With it, the user files and follows tickets without an account. |
| Changelog | A project's list of releases, shown on its status page. |
| Primitive | One of the five parts of a project: monitors, heartbeats, incidents, tickets and the changelog. |
| Tech lead | A user who owns projects. A tech lead sees only their own projects. |
| Supervisor | A user who sees every project and the admin pages and changes nothing. |
| Super admin | A user who invites people and can manage every project. |
