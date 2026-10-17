# Incidents and alerts

An incident records a problem from start to end, sends alerts, and tells users on the status page.

## Automatic incidents

| Source | Opens | Closes |
| --- | --- | --- |
| Monitor | Two failed checks in a row | The first passing check |
| Heartbeat | A missed deadline, or a call to `/fail` | The next ping |

Monita posts the first update, **Investigating**, with the cause. A tech lead can add updates while it is open.

## Manual incidents

Use **Report incident** for a problem no check catches, such as slow sign-in for some users. Pick the monitors and heartbeats it affects, a severity and a first message. A manual incident marks its items as **Issue**, not down, and closes only when a tech lead posts **Resolved**.

## Status updates

Each update has a label and a message:

| Label | Means |
| --- | --- |
| Investigating | We know and we are looking |
| Identified | We found the cause |
| Monitoring | A fix is in; we are watching |
| Resolved | It is over |

Updates show on the status page without the author's name.

## Alert routes

Each project has two routes, one per severity. Open **Settings**, then **Alerts**, to set them.

| Severity | Default | Gets |
| --- | --- | --- |
| Critical | Email and webhook | Incidents of critical monitors and heartbeats |
| Warning | Email only | Incidents of warning items, and new tickets when its webhook is on |

A route takes up to 20 email addresses and one webhook URL. **Send test** checks both.

Alerts leave through an outbox: a slow mail server never delays a check, and a failed delivery is tried again.

## Webhook payload

Monita posts JSON. The `text` field makes it work as-is with Slack and Microsoft Teams incoming webhooks.

```json
{
  "text": "[CRITICAL] Payments / API health is down: HTTP 503, expected 2xx after 2 checks",
  "event": "incident.opened",
  "project": "payments",
  "source": "API health",
  "severity": "critical",
  "cause": "HTTP 503, expected 2xx after 2 checks",
  "incident_url": "https://monita.example.com/projects/payments/incidents/42",
  "at": 1792137600
}
```

| `event` | When |
| --- | --- |
| `incident.opened` | An incident opens |
| `incident.resolved` | It closes. Adds `duration_s`. |
| `ticket.created` | A ticket arrives and the warning route's webhook is on. Adds `ticket_number`, `title` and `ticket_url`. |
| `test` | The test button |

## Daily digest

Once a day, at `MONITA_DIGEST_UTC_HOUR`, each project owner gets one email listing new tickets still in Triage. No email goes out when there are none.
