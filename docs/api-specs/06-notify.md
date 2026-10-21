# 6. Notify

Fields: [../technical-specs/06-data-model.md section 6.3.6](../technical-specs/06-data-model.md#636-alert_routes).

## 6.1 Frontend routes

The React page `/projects/{slug}/settings/alerts` calls these (conventions 1.10). It holds three settings, Critical, Warning and Ticket alerts, each with an on/off switch. A supervisor may read and gets 403 on every write; the read hides each `webhook_url` that is set as `"(hidden)"` (conventions 1.9).

| Method | Path | Access | Input | Behavior | AC |
| --- | --- | --- | --- | --- | --- |
| GET | `/api/ui/projects/{slug}/alerts` | owner, super_admin, supervisor | | One route per severity, critical first: `severity`, `emails` (a list), `webhook_url`, `use_webhook`, `enabled`, `webhook_format`, `webhook_key`. A supervisor reads `(hidden)` for a set `webhook_url` and `webhook_key`. A new project's lists start with the owner's email. | AC-15.01, AC-15.02 |
| PUT | `/api/ui/projects/{slug}/alerts/{severity}` | owner, super_admin | `emails` (one per line, 20 at most), `webhook_url`, `use_webhook` (boolean), `webhook_format` (`json`, `discord`, `telegram`, `telegram_bot`, `ntfy` or `pagerduty`; blank is `json`), `webhook_key` (Telegram's chat ID or PagerDuty's routing key) | Validate each email (ASCII, a name domain, one the SMTP client accepts) and the URL (`http://` or `https://` with a host, no spaces; blank turns it off). `ntfy` needs a topic in the URL. `pagerduty` ignores the URL and stores `https://events.pagerduty.com/v2/enqueue`; it needs an alphanumeric routing key. `telegram` needs a numeric chat ID or an `@channel`. `telegram_bot` (Monita's bot, 6.4) ignores the URL, stores `https://api.telegram.org/`, needs the same chat ID, and is 422 on `webhook_format` while no bot is set and the webhook is ticked. Other formats drop the key. Save, 204. A bad address or too many is 422 on `emails`; a bad URL or a missing ntfy topic on `webhook_url`; an unknown format on `webhook_format`; a missing or bad key on `webhook_key`. (v1.1 Q17) An unknown severity is 404. It does not change `enabled`. | AC-15.01 |
| POST | `/api/ui/projects/{slug}/alerts/{severity}/test` | owner, super_admin | | Queue one test email per recipient and the webhook when it is on. 200 `{sent_to}`, the number queued: recipients, plus one for the webhook. It is 0 when the route has nothing to send to, or when its switch is off. | |
| PUT | `/api/ui/projects/{slug}/alerts/{route}/enabled` | owner, super_admin | `enabled` (boolean) | Switch `critical`, `warning` or `ticket` alerts on or off. Off sends nothing and keeps the recipients and the webhook for when it comes back on. 204. Another route name is 404. | |
| GET | `/api/ui/projects/{slug}/ticket-alerts` | owner, super_admin, supervisor | | `enabled`, `digest` (the owner's daily digest), `webhook_url`, `use_webhook` (a webhook per new ticket) and `webhook_moves` (a webhook per column move). A project with no saved row reads digest on, both webhooks off, `enabled` on. | AC-30.01 |
| PUT | `/api/ui/projects/{slug}/ticket-alerts` | owner, super_admin | `digest`, `webhook_url`, `use_webhook`, `webhook_moves` (booleans and a URL) | Save, 204. The URL is required when `use_webhook` or `webhook_moves` is on, and it must be `http://` or `https://`: 422 on `webhook_url` otherwise. A URL kept while both boxes are off waits for the next time. It does not change `enabled`. | AC-30.03 |
| POST | `/api/ui/projects/{slug}/ticket-alerts/test` | owner, super_admin | | Send one `test` event to the ticket webhook. 200 `{sent_to}`: 1, or 0 when the ticket alerts are off, neither webhook box is on, or no URL is saved. | |
| GET | `/api/ui/projects/{slug}/alerts-preview` | owner, super_admin, supervisor | | Sample alerts of every kind, built by the code that sends the real ones, with this project's name: a list of `{event, label, route, email, webhook}`. `event` is the webhook `event` (6.2). `route` is the setting that sends it: `critical`, `warning` or `ticket`. `email` is `{subject, body}`, or null for a kind that sends no email. | |

Ticket alerts are apart from the severities. The Warning webhook no longer receives tickets.

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
| `incident.opened` | Incident opens | the source's severity route | AC-16.01 |
| `incident.resolved` | Incident closes. Adds `duration_s`. `text` reads "is up", "was deleted" or "was marked resolved by a tech lead". | the severity route | AC-16.02 |
| `source.down` | A monitor or heartbeat fails while its incident switch is off. Adds `source_url` in place of `incident_url`; `text` reads `[DOWN] Payments / <title>: <cause>`. | the severity route | |
| `source.up` | The same source recovers. Adds `duration_s` and `source_url`; `text` reads `[UP] Payments / <source> is up after 4m 12s`. | the severity route | |
| `certificate.expiring` | An HTTPS check sees a certificate that expires within the monitor's `cert_warn_days`, once per certificate (v1.1 Q25). Adds `expires_at` and `source_url`; `severity` is `warning`; `text` reads `[CERT] Payments / <monitor>: the certificate expires in 12 days, on 2026-11-02`, or `expired on` a past day. No incident opens. PagerDuty: a trigger with the dedup key `cert:<monitor id>:<expires_at>`, never resolved. | the warning route | |
| `ticket.created` | A reporter submits a ticket, when the ticket webhook is on. Adds `ticket_number`, `title` and `ticket_url`; `text` reads `[TICKET] Payments #17: <title>`. A ticket the team files sends nothing. | the ticket webhook | AC-30.03 |
| `ticket.moved` | A ticket moves to another column, when "ticket moves" is on. Adds `ticket_number`, `title`, `from`, `to`, `close_reason` (null unless `to` is `done`) and `ticket_url`; `text` reads `[TICKET] Payments #17 moved from In progress to Done (Fixed): <title>`. A move to the same column sends nothing. | the ticket webhook | |
| `test` | Test button. Adds `severity` on the alerts test and `link` on the ticket test. | the tested route | |

A monitor or heartbeat with its alert switch off sends none of the first five (4.3).

## 6.4 Monita's Telegram bot

v1.1 Q33. A super admin sets one bot for the whole Monita. A severity in the `telegram_bot` format posts through it to a chat that a connect link found. The token stays in `settings`: no page, route, outbox row or log line holds it. Delivery queues an outbox row with `channel = 'telegram'` and the chat ID as `recipient`, then adds the token when it sends. `MONITA_TELEGRAM_API` points at another Bot API server.

| Method | Path | Access | Input | Behavior |
| --- | --- | --- | --- | --- |
| GET | `/api/ui/telegram` | session | | `{bot}`: the bot's username without `@`, or null. |
| PUT | `/api/ui/admin/telegram` | super_admin | `token` | Empty forgets the bot. Otherwise the token must look like `123456:ABC-def` (it goes into a URL path); Monita calls `getMe` and `getWebhookInfo`, then saves the token and name and resets the read offset. `{bot}`. A malformed or refused token, or a bot whose updates go to a webhook: 422 on `token`. Telegram unreachable: 422. A supervisor: 403. |
| POST | `/api/ui/projects/{slug}/alerts/{severity}/telegram-link` | owner, super_admin | | A new link for that severity: `{private, group, qr, expires_at}`. `private` is `https://t.me/<bot>?start=<code>`, `group` the same with `startgroup`, `qr` the private link as an SVG. The code is 26 base32 characters; only its SHA-256 is stored, for 10 minutes. A new link replaces the severity's older one. No bot: 422. |
| POST | `/api/ui/projects/{slug}/alerts/{severity}/telegram-link/check` | owner, super_admin | | Reads the bot's new messages (`getUpdates` from the stored offset, `message` updates only), records every `/start <code>` or `/start@<bot> <code>` that opens a live link of any project, and moves the offset past them. `{chat}`: `{chat_id, title}` when this severity's link was opened, which uses the link up; otherwise null. The page then fills the chat ID; nothing is saved until the form is. |

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
