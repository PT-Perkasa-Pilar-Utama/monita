# Alerts

Alerts tell people by email, chat or webhook when a monitor, a heartbeat or a ticket needs them. Each project sets its own alert cards.

Open a project, then **Settings**, then **Alerts**. The old `/alerts` address redirects there.

The page has two sections: **Alert cards** and **Ticket alerts**. A list on the right jumps to each one. Select **How alerts work** at the top for a guide with live samples.

## Alert cards

You pick an **Alert level**, critical or warning, on each monitor and heartbeat. Its incidents go to every alert card that is on and listens to that level.

| Alert level | Use it for |
| --- | --- |
| Critical | An outage that needs someone now |
| Warning | A problem that can wait for working hours |

Every project has two **Owner email** cards, one for critical and one for warning. They email the project's owner and follow the owner when the project changes hands. You cannot edit or delete them; turn one off when someone else should hear instead.

Select **Add alert card** to add your own, up to 10:

| Field | Means |
| --- | --- |
| Global Telegram or Custom | Shows when the global Telegram bot is set up. Custom offers the rest. |
| Sends by | Email, or a webhook format. See the table below. |
| Name | What the card is called on the page and in its test, such as `On call` |
| Listens to | **Critical alerts**, **Warning alerts** or both |
| Email addresses | Email cards only. One per line, up to 20. |
| Webhook URL | Where Monita posts. Not for PagerDuty or the global Telegram bot. |
| Chat ID or Routing key | Telegram and PagerDuty only |

Each card has an **On** or **Off** switch, which saves at once. Off keeps the card and sends nothing. **Send a test** sends one test alert to that card alone. **Edit** changes the card or deletes it.

An address in two cards that hear the same alert gets one email. When no card that is on listens to a level, the page says so. Alerts of that level then go to no one; the incident still opens and shows on the status page.

| Format | URL | Also needs | What Monita posts |
| --- | --- | --- | --- |
| JSON | Any receiver, Slack or Google Chat incoming webhook | | The body under [What recipients get](#what-recipients-get) |
| Discord | The channel's webhook URL | | An embed, posted as Monita: a coloured title that links to Monita (red critical, orange warning, green resolved or up, blue test), the project, and the cause or how long it was down. Mentions such as `@everyone` in the text do not ping. |
| Global Telegram | None: Monita uses the bot a super admin set | A chat, connected with a link | A bold title with the same coloured mark, the project, the cause or how long it was down, and the link to Monita. Telegram makes no link of a `localhost` address; the URL still shows, to copy. |
| Telegram | `https://api.telegram.org/bot<token>/sendMessage` | The chat ID, such as `-1001234567890` or `@team_alerts` | The same as with the global Telegram bot |
| ntfy | The topic URL, such as `https://ntfy.sh/team-alerts` | | A title, the message and a link; priority 5 for critical, 4 for warning, 3 when it resolves |
| PagerDuty | Fixed: the Events API v2 | The routing key of an Events API v2 integration | A trigger when an incident opens or a source goes down, and a resolve with the same dedup key when it ends. **Send a test** sends a trigger and its resolve. |

### Global Telegram

A super admin sets up one bot for every project in **Admin**, **Telegram**. Once it is set, the card form starts with a switch: **Global Telegram** or **Custom**.

1. Select **Add alert card**, pick **Global Telegram**, and select **Connect a chat**.
2. Scan the QR code with your phone, or select **Open in Telegram**, then select **Start**. To alert a group, select **Add to a group** and pick the group.
3. The dialog closes when the bot hears you. The chat fills in. Select **Add card**.

The link works for 10 minutes. A bot cannot send to a person by @username, so the link is how Monita learns the chat. A public channel can also be typed as `@channel`. Add the bot to the channel as an admin first.

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
| Global Telegram or Custom, then the format | Where the webhook goes: JSON, Discord, Telegram, ntfy or the global Telegram bot. Not PagerDuty, because a ticket is not an incident. |
| Webhook URL or chat | Required when either webhook box is ticked |

The new-ticket and move alerts go to this webhook, not to any alert card. Discord and Telegram get them laid out as for incidents. **Send a test** works when either webhook box is saved as on.

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
| `test` | A card's or the ticket test button | The card or section tested |

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
