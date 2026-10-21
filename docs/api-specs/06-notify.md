# 6. Notify

Fields: [../technical-specs/06-data-model.md section 6.3.6](../technical-specs/06-data-model.md#636-alert_cards).

## 6.1 Frontend routes

The React page `/projects/{slug}/settings/alerts` calls these (conventions 1.10). It holds the project's alert cards, then Ticket alerts (v1.1 Q34). A card sends the alerts of the levels it listens to: `critical`, `warning` or both. Every project has two fixed cards of kind `owner_email`, one per level, which email the project's owner, looked up when an alert goes out; only their switch changes. A project adds up to 10 of its own: `email` (an address list) or a webhook in one format. An alert goes to every card that is on and listens to its level, and an address in several such cards gets one email. With no card listening, the alert goes nowhere and nothing fails: the incident, the check and the ping go on. A supervisor may read and gets 403 on every write; the read hides each set `url` and `key` as `"(hidden)"` (conventions 1.9).

| Method | Path | Access | Input | Behavior | AC |
| --- | --- | --- | --- | --- | --- |
| GET | `/api/ui/projects/{slug}/alert-cards` | owner, super_admin, supervisor | | The cards, the owner's first (critical, then warning), then the project's own in the order they were added: `{id, kind, name, critical, warning, enabled, emails, url, key, fixed}`. `fixed` is true for the owner's two. | AC-15.01, AC-15.02 |
| POST | `/api/ui/projects/{slug}/alert-cards` | owner, super_admin | `kind` (`email`, `json`, `discord`, `telegram`, `telegram_bot`, `ntfy` or `pagerduty`), `name` (1 to 60 characters), `critical`, `warning` (booleans; at least one), `emails` (one a line or comma-separated, 1 to 20, for `email`), `url`, `key` | A new card, on. 201 `{id}`. An `email` card keeps only its addresses; a webhook card needs its URL (`http://` or `https://` with a host, no spaces) and keeps its key. `ntfy` needs a topic in the URL. `pagerduty` ignores the URL, stores `https://events.pagerduty.com/v2/enqueue` and needs an alphanumeric routing key. `telegram` and `telegram_bot` need a numeric chat ID or an `@channel`; `telegram_bot` stores `https://api.telegram.org/` (6.4). Other formats drop the key. 422 names the field: `kind` (unknown, `owner_email`, or `telegram_bot` with no bot set), `name`, `levels`, `emails`, `url`, `key`. A project with 10 cards of its own: 422. | AC-15.01 |
| PUT | `/api/ui/projects/{slug}/alert-cards/{id}` | owner, super_admin | as POST | Saves the card, 204. The owner's cards: 422. Another project's card or none: 404. It does not change `enabled`. | |
| DELETE | `/api/ui/projects/{slug}/alert-cards/{id}` | owner, super_admin | | 204. The owner's cards: 422. | |
| PUT | `/api/ui/projects/{slug}/alert-cards/{id}/enabled` | owner, super_admin | `enabled` (boolean) | Any card, the owner's too. Off sends nothing and keeps the card. 204, or 404. | |
| POST | `/api/ui/projects/{slug}/alert-cards/{id}/test` | owner, super_admin | | One `test` alert to this card alone, on or off. 200 `{sent_to}`: its addresses, or 1 for a webhook card. A PagerDuty test sends a trigger and its resolve. | |
| PUT | `/api/ui/projects/{slug}/ticket-alerts/enabled` | owner, super_admin | `enabled` (boolean) | Ticket alerts on or off; their settings stay. 204. | |
| GET | `/api/ui/projects/{slug}/ticket-alerts` | owner, super_admin, supervisor | | `enabled`, `digest` (the owner's daily digest), `webhook_url`, `use_webhook` (a webhook per new ticket) `webhook_moves` (a webhook per column move), `webhook_format` and `webhook_key`. A supervisor reads `(hidden)` for a set URL and key. A project with no saved row reads digest on, both webhooks off, `enabled` on. | AC-30.01 |
| PUT | `/api/ui/projects/{slug}/ticket-alerts` | owner, super_admin | `digest`, `webhook_url`, `use_webhook`, `webhook_moves` (booleans and a URL), `webhook_format` (`json`, `discord`, `telegram`, `telegram_bot` or `ntfy`; blank is `json`; v1.1 Q36), `webhook_key` (a Telegram chat ID) | Save, 204. The format and key are checked as for a card; PagerDuty is refused on `webhook_format`, and so is `telegram_bot` while no global bot is set. Discord and Telegram get the ticket as a card. The URL is required when `use_webhook` or `webhook_moves` is on, and it must be `http://` or `https://`: 422 on `webhook_url` otherwise. A URL kept while both boxes are off waits for the next time. It does not change `enabled`. | AC-30.03 |
| POST | `/api/ui/projects/{slug}/ticket-alerts/test` | owner, super_admin | | Send one `test` event to the ticket webhook. 200 `{sent_to}`: 1, or 0 when the ticket alerts are off, neither webhook box is on, or no URL is saved. | |
| GET | `/api/ui/projects/{slug}/alerts-preview` | owner, super_admin, supervisor | | Sample alerts of every kind, built by the code that sends the real ones, with this project's name: a list of `{event, label, route, email, webhook}`. `event` is the webhook `event` (6.2). `route` is the setting that sends it: `critical`, `warning` or `ticket`. `email` is `{subject, body}`, or null for a kind that sends no email. | |

Ticket alerts are apart from the alert cards. No card receives tickets.

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
| `ticket.created` | A reporter submits a ticket, when the ticket webhook is on. Adds `ticket_number`, `title` and `ticket_url`; `text` reads `[TICKET] Payments #17: <title>`. A ticket the team files sends nothing. | the ticket webhook | AC-30.03 |
| `ticket.moved` | A ticket moves to another column, when "ticket moves" is on. Adds `ticket_number`, `title`, `from`, `to`, `close_reason` (null unless `to` is `done`) and `ticket_url`; `text` reads `[TICKET] Payments #17 moved from In progress to Done (Fixed): <title>`. A move to the same column sends nothing. | the ticket webhook | |
| `test` | Test button. Adds `card` (the card's name) on a card's test and `link` on the ticket test. | the tested card | |

A monitor or heartbeat with its alert switch off sends none of the first five (4.3).

## 6.4 The global Telegram bot

v1.1 Q33. A super admin sets one bot for the whole Monita. A severity in the `telegram_bot` format posts through it to a chat that a connect link found. The token stays in `settings`: no page, route, outbox row or log line holds it. Delivery queues an outbox row with `channel = 'telegram'` and the chat ID as `recipient`, then adds the token when it sends. `MONITA_TELEGRAM_API` points at another Bot API server.

| Method | Path | Access | Input | Behavior |
| --- | --- | --- | --- | --- |
| GET | `/api/ui/telegram` | session | | `{bot}`: the bot's username without `@`, or null. |
| PUT | `/api/ui/admin/telegram` | super_admin | `token` | Empty forgets the bot. Otherwise the token must look like `123456:ABC-def` (it goes into a URL path); Monita calls `getMe` and `getWebhookInfo`, then saves the token and name and resets the read offset. `{bot}`. A malformed or refused token, or a bot whose updates go to a webhook: 422 on `token`. Telegram unreachable: 422. A supervisor: 403. |
| POST | `/api/ui/projects/{slug}/telegram-link` | owner, super_admin | | A new link for a card being added or edited: `{code, private, group, qr, expires_at}`. `private` is `https://t.me/<bot>?start=<code>`, `group` the same with `startgroup`, `qr` the private link as an SVG. The code is 26 base32 characters; only its SHA-256 is stored, for 10 minutes. A project has one open link; a new one replaces it. No bot: 422. |
| POST | `/api/ui/projects/{slug}/telegram-link/check` | owner, super_admin | `code` | Reads the bot's new messages (`getUpdates` from the stored offset, `message` updates only), records every `/start <code>` or `/start@<bot> <code>` that opens a live link of any project, and moves the offset past them. `{chat}`: `{chat_id, title}` when the link with `code` was opened, which uses it up; otherwise null. The card form then fills the chat ID; nothing is saved until the form is. |

## 6.3 Outgoing email

| Event | Subject |
| --- | --- |
| Opened | `[CRITICAL] Payments / API health is down` |
| Resolved | `[RESOLVED] Payments / API health is up after 4m 12s` |
| Down (incident switch off) | `[DOWN] Payments / API health is down` |
| Up (incident switch off) | `[UP] Payments / API health is up after 4m 12s` |
| Digest | `[Monita] Payments: 3 new tickets` |
| Invite | `You are invited to Monita` |
| Test | `[TEST] Payments: alerts for critical reach this channel.` |
| Mail test | `Monita test email` |

Plain text body with the cause and the incident link (AC-16.01), or the monitor or heartbeat page for Down and Up. The digest goes to the project owner when the owner is not disabled and Ticket alerts have the digest on. It lists number, title and link of tickets created after the previous digest (a ticket-id watermark in `settings`) and still in Triage (AC-30.01), never the description, and is not sent when there are none (AC-30.02). It runs once a day at `MONITA_DIGEST_UTC_HOUR`. A ticket sends no other email.
