# Glossary

Monita's docs, pages and code use each of these words with one meaning. Say "tech lead", not "TL", and "ticket board", not "kanban". The scope decision log defines the first set in [Terms](decisions/2026-10-16-monita-v1-scope.md#terms).

| Term | Meaning |
| --- | --- |
| Project | One system your team runs. It has its own monitors, heartbeats, status page and tickets. |
| Monitor | A URL that Monita requests on a fixed interval. |
| Check | One request that a monitor makes. It passes or fails. |
| Heartbeat | A URL that a cron job calls when it runs. Monita expects the call before a deadline. |
| Ping | One call from a cron job to its heartbeat URL. |
| Incident | A record that something is down. Monita opens an automatic incident itself. A tech lead or a super admin opens a manual incident by hand. |
| Alert | An email or a webhook that Monita sends when an incident opens or closes. |
| Webhook | A message that Monita posts to a URL, for example a Slack or Teams channel. |
| Status page | The public page of a project. Anyone with the link can read it. |
| Ticket | A bug report from a user. It moves through triage, backlog, in progress and done. |
| Ticket board | The tech lead's view of every ticket in a project. The public board on the status page leaves out tickets in triage. |
| Ref ID | A code that a project's backend gives one of its users. With it, the user files and follows tickets without an account. |
| Changelog | A project's list of releases, shown on its status page. |
| Primitive | One of the five parts of a project: monitors, heartbeats, incidents, tickets and the changelog. |
| Tech lead | A user who owns projects. A tech lead sees only their own projects. |
| Supervisor | A user who sees every project and the admin pages and changes nothing. |
| Super admin | A user who invites people and can manage every project. |
