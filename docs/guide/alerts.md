# Alerts

Alerts tell people by email and webhook when a monitor, a heartbeat or a ticket needs them. Each project sets its own recipients.

Open a project, then **Settings**, then **Alerts**. The old `/alerts` address redirects there.

The page has three sections: **Critical alerts**, **Warning alerts** and **Ticket alerts**. A list on the right jumps to each one. Select **How alerts work** at the top for a guide with live samples.

## The on and off switch

Each section has an **On** or **Off** switch in its header. The switch saves at once.

- Off means Monita sends nothing for that section.
- The recipients stay for when you turn it back on.
- **Send a test** is disabled while a section is off or has unsaved changes.

## Critical and warning

You pick an **Alert level**, critical or warning, on each monitor and heartbeat. Its incidents alert that level's recipients.

| Alert level | Use it for | Sends by default |
| --- | --- | --- |
| Critical | An outage that needs someone now | Email |
| Warning | A problem that can wait for working hours | Email |

Neither level calls a webhook until you tick **Also send to a webhook** and set its URL.

Each section has these fields:

| Field | Means |
| --- | --- |
| Email addresses | One per line, up to 20. A new project starts with its owner's email in both sections. |
| Also send to a webhook | Tick it to add a webhook |
| Format | How the webhook speaks. See the table below. |
| Webhook URL | Where Monita posts. Required when the box is ticked, except for PagerDuty. |
| Chat ID or Routing key | Telegram and PagerDuty only |

| Format | URL | Also needs | What Monita posts |
| --- | --- | --- | --- |
| JSON | Any receiver, Slack or Google Chat incoming webhook | | The body under [What recipients get](#what-recipients-get) |
| Discord | The channel's webhook URL | | `content` with the alert line and a link. Mentions such as `@everyone` in the text do not ping. |
| Telegram with Monita's bot | None: Monita uses the bot a super admin set | A chat, connected with a link | The alert line and a link |
| Telegram with your own bot | `https://api.telegram.org/bot<token>/sendMessage` | The chat ID, such as `-1001234567890` or `@team_alerts` | The alert line and a link |
| ntfy | The topic URL, such as `https://ntfy.sh/team-alerts` | | A title, the message and a link; priority 5 for critical, 4 for warning, 3 when it resolves |
| PagerDuty | Fixed: the Events API v2 | The routing key of an Events API v2 integration | A trigger when an incident opens or a source goes down, and a resolve with the same dedup key when it ends. **Send a test** sends a trigger and its resolve. |

### Telegram with Monita's bot

The format shows once a super admin sets the bot in **Admin**, **Telegram**.

1. Pick **Telegram with Monita's bot** and select **Connect a chat**.
2. Scan the QR code with your phone, or select **Open in Telegram**, then select **Start**. To alert a group, select **Add to a group** and pick the group.
3. The dialog closes when the bot hears you. The chat fills in. Select **Save changes**.

The link works for 10 minutes. A bot cannot send to a person by @username, so the link is how Monita learns the chat. A public channel can also be typed as `@channel`; add the bot to the channel as an admin first.

Select **Save changes**. Then select **Send a test** to send a test alert to the saved recipients. A section with no saved recipient sends nothing and says so.

Each recipient gets one alert when an incident opens and one when it closes. A manual incident sends no alert. It shows on the status page.

## Alerts without an incident

A monitor or heartbeat can have **Open an incident** off. Then a failure sends a down alert, and recovery sends an up alert. The status page shows nothing. The team's lists show **Down**.

A monitor or heartbeat with **Send alerts** off sends nothing at all.

## Ticket alerts

This section covers user bug reports and moves on the ticket board. A ticket your team files sends no new-ticket alert.

| Box | Means |
| --- | --- |
| Email `<owner>` a daily list of new tickets in Triage | The digest. On by default. It goes to the project owner. |
| Send each new ticket to the webhook | Off by default |
| Send each ticket move to the webhook | Off by default |
| Webhook URL | Required when either webhook box is ticked |

The new-ticket and move alerts go to this webhook, not to the warning webhook. **Send a test** works when either webhook box is saved as on.

## What recipients get

Select **How alerts work**, then **What recipients get**. Pick an alert to see the email and the webhook body that Monita builds for your project. The incident and the ticket in the samples are made up. The webhook samples show your own webhook URL.

| `event` | When | Route |
| --- | --- | --- |
| `incident.opened` | An incident opens | Critical or warning |
| `incident.resolved` | It closes. Adds `duration_s`. | Critical or warning |
| `source.down` | A source with incidents off fails. Adds `source_url`. | Critical or warning |
| `source.up` | That source recovers. Adds `duration_s` and `source_url`. | Critical or warning |
| `certificate.expiring` | An HTTPS monitor's certificate expires within its warning window. Once per certificate. Adds `expires_at` and `source_url`. | Warning |
| `ticket.created` | A user files a ticket. Adds `ticket_number`, `title` and `ticket_url`. | Ticket |
| `ticket.moved` | A ticket changes column. Adds `ticket_number`, `title`, `ticket_url`, `from`, `to` and `close_reason`. | Ticket |
| `test` | The test button | The section tested |

Every body has `text`, `event`, `project`, `at` (Unix seconds, UTC) and, for incidents, `source`, `severity` and `cause`. The `text` field makes the body work as-is with Slack and Google Chat incoming webhooks. In a ticket's `text`, Monita writes `&`, `<` and `>` as `&amp;`, `&lt;` and `&gt;`, so a reporter's title cannot add a link or an `@channel` mention. The `title` field keeps the words as typed.

```json
{
  "text": "[CRITICAL] Payments / Checkout API is down: HTTP 503 on 2 checks in a row",
  "event": "incident.opened",
  "project": "payments-k3x9qa",
  "source": "Checkout API",
  "severity": "critical",
  "cause": "HTTP 503 on 2 checks in a row",
  "incident_url": "https://monita.example.com/projects/payments-k3x9qa/incidents/42",
  "at": 1792137600
}
```

A moved ticket sends this:

```json
{
  "text": "[TICKET] Payments #17 moved from In progress to Done (Fixed): Export to Excel drops the last row",
  "event": "ticket.moved",
  "project": "payments-k3x9qa",
  "ticket_number": 17,
  "title": "Export to Excel drops the last row",
  "ticket_url": "https://monita.example.com/projects/payments-k3x9qa/tickets/17",
  "from": "in_progress",
  "to": "done",
  "close_reason": "fixed",
  "at": 1792137600
}
```

A new ticket and a move send no email. Only the webhook gets them. The owner's daily digest lists new tickets.

## Delivery

- Alerts leave from a queue, so a slow mail server or webhook never delays a check.
- Monita sends a POST with a JSON body.
- A 2xx answer within 10 seconds counts as delivered. A redirect counts as a failure.
- Monita tries a failed call again after 1, 5 and 15 minutes. After the fourth failure it gives up and writes the error to its log.
- Monita leaves the webhook URL out of its logs and error messages. Treat the URL as a secret.
- A supervisor sees `(hidden)` instead of a webhook URL.
- A super admin sets the mail server on [Outgoing email](admin.md#outgoing-email).

Monita keeps delivered alerts for 30 days.
