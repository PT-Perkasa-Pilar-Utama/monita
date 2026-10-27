# 6. Notify

Fields: [../technical-specs/06-data-model.md section 6.3.6](../technical-specs/06-data-model.md#636-alert_cards).

## 6.1 Frontend routes

The React page `/projects/{slug}/settings/alerts` calls these (conventions 1.10). It holds the project's alert cards, then its ticket cards (v1.1 Q34, Q37). A card belongs to one section and never to both. An incident card sends the alerts of the levels it listens to: `critical`, `warning` or both. A ticket card sends new tickets, ticket moves or both, and the owner's card can send the daily digest. Every project has five fixed cards of kind `owner_email`, which email the project's owner, looked up when an alert goes out. Two are incident cards, critical and warning, and they start on. Three are ticket cards, Daily digest, New tickets and Ticket moves, and they start off. Only their switch changes. A project adds up to 10 of its own in each section: `email` (an address list) or a webhook in one format. An alert goes to every card that is on and listens to it, and an address in several such cards gets one email. With no card listening, the alert goes nowhere and nothing fails: the incident, the check and the ping go on. A supervisor may read and gets 403 on every write; the read hides each set `url` and `key` as `"(hidden)"` (conventions 1.9).

| Method | Path | Access | Input | Behavior | AC |
| --- | --- | --- | --- | --- | --- |
| GET | `/api/ui/projects/{slug}/alert-cards` | owner, super_admin, supervisor | | The incident cards, then the ticket cards. In each section the owner's cards come first (critical, warning; digest, new tickets, moves), then the project's own in the order they were added: `{id, kind, name, critical, warning, new_tickets, ticket_moves, digest, section, enabled, emails, url, key, fixed}`. `section` is `incidents` or `tickets`, from what the card listens to. `fixed` is true for the owner's five. | AC-15.01, AC-15.02, AC-30.01 |
| POST | `/api/ui/projects/{slug}/alert-cards` | owner, super_admin | `kind` (`email`, `json`, `discord`, `telegram`, `telegram_bot`, `ntfy` or `pagerduty`), `section` (`incidents`, the default, or `tickets`), `name` (optional, up to 60 characters), `critical`, `warning` (booleans; at least one in an incident card), `new_tickets`, `ticket_moves` (booleans; at least one in a ticket card), `emails` (one a line or comma-separated, 1 to 20, for `email`), `url`, `key` | A new card, on. 201 `{id}`. An `email` card keeps only its addresses; a webhook card needs its URL (`http://` or `https://` with a host, no spaces) and keeps its key. `ntfy` needs a topic in the URL. `pagerduty` ignores the URL, stores `https://events.pagerduty.com/v2/enqueue` and needs an alphanumeric routing key. `telegram` needs a numeric chat ID or an `@channel`. `telegram_bot` needs a chat that one of the project's own connect links found, and stores `https://api.telegram.org/` (6.4). Other formats drop the key. With no `name`, the server names the card after its kind: Email, JSON webhook, Discord, Telegram, Global Telegram, ntfy or PagerDuty. A `telegram_bot` card is named after its chat's title. A ticket card ignores `critical` and `warning`. An incident card ignores the ticket flags. A ticket card cannot be `pagerduty`: 422 on `kind`, "PagerDuty takes incidents only, so pick another kind for tickets." A ticket card with neither flag: 422 on `events`, "Pick new tickets, ticket moves or both." 422 names the field: `kind` (unknown, `owner_email`, `pagerduty` for tickets, or `telegram_bot` with no bot set), `name`, `levels`, `events`, `emails`, `url`, `key` (also a `telegram_bot` chat the project never connected: "Connect this chat with a link first, so Monita knows your team added the bot there."). A section with 10 cards of its own: 422, "This section has 10 cards, which is the limit, so delete one first." | AC-15.01, AC-30.03 |
| PUT | `/api/ui/projects/{slug}/alert-cards/{id}` | owner, super_admin | as POST | Saves the card, 204. The owner's cards: 422. Another project's card or none: 404. A `section` other than the card's own: 404, so an update cannot move a card to the other section. A card that keeps its kind keeps its name when `name` is empty. It does not change `enabled`. | |
| DELETE | `/api/ui/projects/{slug}/alert-cards/{id}` | owner, super_admin | | 204. The owner's cards: 422. | |
| PUT | `/api/ui/projects/{slug}/alert-cards/{id}/enabled` | owner, super_admin | `enabled` (boolean) | Any card, the owner's too. Off sends nothing and keeps the card. 204, or 404. | |
| POST | `/api/ui/projects/{slug}/alert-cards/{id}/test` | owner, super_admin | | Six a minute per project, from all its cards together; past that, 429. One `test` alert to this card alone, on or off. 200 `{sent_to}`: its addresses, or 1 for a webhook card. A PagerDuty test sends a trigger and its resolve. | |
| GET | `/api/ui/projects/{slug}/alerts-preview` | owner, super_admin, supervisor | | Sample alerts of every kind, built by the code that sends the real ones, with this project's name: a list of `{event, label, route, email, webhook}`. `event` is the webhook `event` (6.2). `route` is the section that sends it: `critical`, `warning` or `ticket`. `email` is `{subject, body}`, or null for a kind that sends no email. | |

## 6.2 Outgoing webhook

`POST {webhook_url}`, `Content-Type: application/json`, 10 s timeout. Any 2xx is success. Retries: 1, 5, 15 min.

```json
{
  "text": "[CRITICAL] Payments / API health is down: HTTP 503, expected 2xx after 2 checks",
  "event": "incident.opened",
  "project": "payments",
  "source": "API health",
  "severity": "critical",
  "cause": "HTTP 503, expected 2xx after 2 checks",
  "incident_url": "https://monita.company.co.id/projects/payments/incidents/42",
  "at": 1792137600
}
```

| `event` | When | Sent to | AC |
| --- | --- | --- | --- |
| `incident.opened` | Incident opens | every card that listens to the source's level | AC-16.01 |
| `incident.resolved` | Incident closes. Adds `duration_s`. `text` reads "is up", "was deleted" or "was marked resolved by a tech lead". | the same cards | AC-16.02 |
| `source.down` | A monitor or heartbeat fails while its incident switch is off. Adds `source_url` in place of `incident_url`; `text` reads `[DOWN] Payments / <title>: <cause>`. | the same cards | |
| `source.up` | The same source recovers. Adds `duration_s` and `source_url`; `text` reads `[UP] Payments / <source> is up after 4m 12s`. | the same cards | |
| `certificate.expiring` | An HTTPS check sees a certificate that expires within the monitor's `cert_warn_days`, once per certificate (v1.1 Q25). Adds `expires_at` and `source_url`; `severity` is `warning`; `text` reads `[CERT] Payments / <monitor>: the certificate expires in 12 days, on 2026-11-02`, or `expired on` a past day. No incident opens. PagerDuty: a trigger with the dedup key `cert:<monitor id>:<expires_at>`, never resolved. | every card that listens to warning | |
| `ticket.created` | A reporter submits a ticket. Adds `ticket_number`, `title` and `ticket_url`; `text` reads `[TICKET] Payments #17: <title>`. A ticket the team files sends nothing. | every ticket card that is on and listens to new tickets | AC-30.03 |
| `ticket.moved` | A ticket moves to another column. Adds `ticket_number`, `title`, `from`, `to`, `close_reason` (null unless `to` is `done`) and `ticket_url`; `text` reads `[TICKET] Payments #17 moved from In progress to Done (Fixed): <title>`. A move to the same column sends nothing. | every ticket card that is on and listens to ticket moves | |
| `test` | Test button. Adds `card` (the card's name). | the tested card | |

A monitor or heartbeat with its alert switch off sends none of the first five (4.3).

## 6.4 The global Telegram bot

v1.1 Q33. A super admin sets one bot for the whole Monita. A card in the `telegram_bot` format posts through it to a chat that one of the project's own connect links found. A heard link records its chat in `telegram_chats` for that project, so a tech lead cannot point the shared bot at a chat their team never added it to. Every create and edit of a `telegram_bot` card checks this. The token stays in `settings`: no page, route, outbox row or log line holds it. Delivery queues an outbox row with `channel = 'telegram'` and the chat ID as `recipient`, then adds the token when it sends. `MONITA_TELEGRAM_API` points at another Bot API server.

| Method | Path | Access | Input | Behavior |
| --- | --- | --- | --- | --- |
| GET | `/api/ui/telegram` | session | | `{bot}`: the bot's username without `@`, or null. |
| PUT | `/api/ui/admin/telegram` | super_admin | `token` | Empty forgets the bot. Otherwise the token must look like `123456:ABC-def` (it goes into a URL path); Monita calls `getMe` and `getWebhookInfo`, then saves the token and name and resets the read offset. `{bot}`. A malformed or refused token, or a bot whose updates go to a webhook: 422 on `token`. Telegram unreachable: 422. A supervisor: 403. |
| POST | `/api/ui/projects/{slug}/telegram-link` | owner, super_admin | | A new link for a card being added or edited: `{code, private, group, qr, expires_at}`. `private` is `https://t.me/<bot>?start=<code>`, `group` the same with `startgroup`, `qr` the private link as an SVG. The code is 26 base32 characters; only its SHA-256 is stored, for 10 minutes. A project has one open link; a new one replaces it. No bot: 422. |
| POST | `/api/ui/projects/{slug}/telegram-link/check` | owner, super_admin | `code` | Reads the bot's new messages (`getUpdates` from the stored offset, `message` and `channel_post` updates), records every `/start <code>` or `/start@<bot> <code>` that opens a live link of any project, and moves the offset past them. `{chat}`: `{chat_id, title}` when the link with `code` was opened, which uses it up and proves the chat for the project; otherwise null. The card form then fills the chat ID; nothing is saved until the form is. |

## 6.3 Outgoing email

| Event | Subject |
| --- | --- |
| Opened | `[CRITICAL] Payments / API health is down` |
| Resolved | `[RESOLVED] Payments / API health is up after 4m 12s` |
| Down (incident switch off) | `[DOWN] Payments / API health is down` |
| Up (incident switch off) | `[UP] Payments / API health is up after 4m 12s` |
| Digest | `[Monita] Payments: 3 new tickets` |
| New ticket | `[TICKET] Payments / #17 Export to Excel drops the last row` |
| Ticket moved | `[TICKET] Payments / #17 moved to Done (Fixed)` |
| Invite | `You are invited to Monita` |
| Test | `[TEST] Payments: alerts for critical reach this channel.` |
| Mail test | `Monita test email` |

At delivery, an email whose recipient equals Monita's own sender address is deleted unsent. The sender is the saved mail server's From, else `MONITA_MAIL_FROM`, and the comparison ignores case. An active user who signs in with that address still gets mail. The rule covers every email in the outbox: alerts, digests, invites and card tests. The admin test email (4.4, `test-send`) goes straight to the server, so it is not affected.

Plain text body with the cause and the incident link (AC-16.01), or the monitor or heartbeat page for Down and Up. The digest goes to the project owner when the owner is not disabled and the owner's Daily digest card is on (off by default for a new project, v1.1 Q37). It lists number, title and link of tickets created after the previous digest (a ticket-id watermark in `settings`) and still in Triage (AC-30.01), never the description, and is not sent when there are none (AC-30.02). It runs once a day at `MONITA_DIGEST_UTC_HOUR`. A new ticket and a move send an email to each ticket card that listens: the title, the From and To lines for a move, and `Open the ticket: {url}`. The owner's cards email the owner. An email card gets one email for each ticket, and an address in several cards gets one email.
