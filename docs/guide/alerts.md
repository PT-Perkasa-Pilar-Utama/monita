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

Every project has two **Owner email** cards, one for critical and one for warning. They email the project's owner and follow the owner when the project changes hands. You cannot edit or delete them. Turn one off when someone else should hear instead.

Select **Add alert card** to add your own, up to 10. Monita names a card after its kind, such as `Discord`. A global Telegram card takes its chat's name.


| Field | Means |
| --- | --- |
| Global Telegram or Custom | Shows when the global Telegram bot is set up. Custom offers the rest. |
| Sends by | Email, or a webhook format. See the table below. |
| Listens to | **Critical alerts**, **Warning alerts** or both |
| Email addresses | Email cards only. One per line, up to 20. |
| Webhook URL | Where Monita posts. Not for PagerDuty or the global Telegram bot. |
| Chat ID or Routing key | Telegram and PagerDuty only |

Each card has an **On** or **Off** switch, which saves at once. Off keeps the card and sends nothing. **Send a test** sends one test alert to that card alone. **Edit** changes the card or deletes it.

An address in two cards that hear the same alert gets one email. When no card that is on listens to a level, the page says so. Alerts of that level then go to no one. The incident still opens and shows on the status page.

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

1. Select **Add alert card** and pick **Global Telegram**. The form shows a connect link at once.
2. Scan the QR code with your phone, or select **Open in Telegram**, then select **Start**. To alert a group, select **Add @bot to a group** and pick the group.
3. The form checks every 3 seconds. When the bot hears you, it shows **Connected to** and the chat's name. Select **Change** to pick another chat. Select **Add card**.

To alert a channel, make the bot an admin of the channel. Then post the `/start` line that the form shows in the channel.

The link works for 10 minutes. After that, select **Get a new link**. The link is the only way to pick a chat for the global bot. It shows that your team added the bot to that chat, so no project can make the shared bot post in a chat that is not its own.

Each recipient gets one alert when an incident opens and one when it closes. A manual incident sends no alert. It shows on the status page.

## Alerts without an incident

A monitor or heartbeat can have **Open an incident** off. Then a failure sends a down alert, and recovery sends an up alert. The status page shows nothing. The team's lists show **Down**.

A monitor or heartbeat with **Send alerts** off sends nothing at all.

## Ticket alerts

This section holds ticket cards. A ticket card sends new bug reports from your users, each move on the board, or both. A ticket your team files sends no new-ticket alert.

Every project has three fixed **Owner email** ticket cards. They are off by default for a new project. Turn one on to use it. You cannot edit or delete them.

| Card | Sends |
| --- | --- |
| Daily digest | One email a day to the project owner. It lists new tickets in Triage. |
| New tickets | An email to the owner for each new ticket |
| Ticket moves | An email to the owner each time a ticket changes column |

Select **Add ticket card** to add your own, up to 10. The form is the same as for alert cards, with these differences:

| Field | Means |
| --- | --- |
| Sends by | Email, JSON, Discord, Telegram, Global Telegram or ntfy. PagerDuty is not offered, because a ticket never resolves. |
| Listens to | **New tickets**, **Ticket moves** or both |

Each ticket card has an **On** or **Off** switch and **Send a test**. An email list gets one email for each ticket. An address in several cards gets one email. A card shows badges for what it sends: **Daily digest**, **New tickets** or **Ticket moves**.

## What recipients get

Select **How alerts work**, then **What recipients get**. Pick an alert to see the email and the webhook body that Monita builds for your project. The incident and the ticket in the samples are made up. The webhook samples show your own webhook URL.

| `event` | When | Route |
| --- | --- | --- |
| `incident.opened` | An incident opens | Critical or warning |
| `incident.resolved` | It closes. Adds `duration_s`. | Critical or warning |
| `source.down` | A source with incidents off fails. Adds `source_url`. | Critical or warning |
| `source.up` | That source recovers. Adds `duration_s` and `source_url`. | Critical or warning |
| `certificate.expiring` | An HTTPS monitor's certificate expires within its warning window. Once per certificate. Adds `expires_at` and `source_url`. | Warning |
| `ticket.created` | A user files a ticket. Adds `ticket_number`, `title` and `ticket_url`. | Ticket cards |
| `ticket.moved` | A ticket changes column. Adds `ticket_number`, `title`, `ticket_url`, `from`, `to` and `close_reason`. | Ticket cards |
| `test` | A card's test button | The card tested |

Every body has `text`, `event`, `project`, `at` (Unix seconds, UTC) and, for incidents, `source`, `severity` and `cause`. The `text` field makes the body work as-is with Slack and Google Chat incoming webhooks. In every `text`, Monita writes `&`, `<` and `>` as `&amp;`, `&lt;` and `&gt;`, so a reporter's title or the words a job sends with `/fail` cannot add a link or an `@channel` mention. The `title` and `cause` fields keep the words as typed, and so do the email, ntfy and PagerDuty. A Discord card writes square brackets with a backslash, so a cause or a title cannot hide a link behind words.

```json
{
  "text": "[CRITICAL] Payments / Checkout API is down: HTTP 503, expected 2xx after 2 checks",
  "event": "incident.opened",
  "project": "payments-k3x9qa",
  "source": "Checkout API",
  "severity": "critical",
  "cause": "HTTP 503, expected 2xx after 2 checks",
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

A new ticket sends an email with the subject `[TICKET] {project} / #{n} {title}`. A move sends `[TICKET] {project} / #{n} moved to {Stage}`, and a close adds the reason, such as `Done (Fixed)`. The body has the title and a link: **Open the ticket**. A move also shows the From and To columns.

## Delivery

- Alerts leave from a queue, so a slow mail server or webhook never delays a check.
- Monita sends a POST. A JSON card sends the body above. Discord, Telegram, ntfy and PagerDuty cards send the body each service expects.
- A 2xx answer within 10 seconds counts as delivered. A redirect counts as a failure.
- Monita tries a failed call again after 1, 5 and 15 minutes. After the fourth failure it gives up and writes the error to its log.
- Monita leaves the webhook URL out of its logs and error messages. Treat the URL as a secret.
- A supervisor sees `(hidden)` instead of a webhook URL.
- A super admin sets the mail server on [Outgoing email](admin.md#outgoing-email).
- A project sends at most 6 test alerts a minute.
- A webhook server whose last send got no answer gets one alert a delivery pass, and its other alerts wait a few seconds, so one dead server cannot hold up other projects' alerts.

Monita keeps delivered alerts for 30 days.
