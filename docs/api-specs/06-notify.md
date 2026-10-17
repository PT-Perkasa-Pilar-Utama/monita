# 6. Notify

Fields: [../technical-specs/06-data-model.md section 6.3.6](../technical-specs/06-data-model.md#636-alert_routes).

## 6.1 Frontend routes

The React page `/projects/{slug}/alerts` calls these (conventions 1.10).

| Method | Path | Access | Input | Behavior | AC |
| --- | --- | --- | --- | --- | --- |
| GET | `/api/ui/projects/{slug}/alerts` | owner, super_admin | | One route per severity, critical first: `severity`, `emails`, `webhook_url`, `use_webhook`. | AC-15.01, AC-15.02 |
| PUT | `/api/ui/projects/{slug}/alerts/{severity}` | owner, super_admin | `emails` (one per line), `webhook_url`, `use_webhook` (boolean) | Validate each email (ASCII, a name domain, one the SMTP client accepts) and the URL (`http://` or `https://` with a host, no spaces). Save, 204. A bad address is 422 on `emails`; a bad URL on `webhook_url`. An unknown severity is 404. | AC-15.01 |
| POST | `/api/ui/projects/{slug}/alerts/{severity}/test` | owner, super_admin | | Queue one test email per recipient and the webhook. 200 `{sent_to}`, the number of recipients. | |

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

| `event` | When | AC |
| --- | --- | --- |
| `incident.opened` | Incident opens | AC-16.01 |
| `incident.resolved` | Incident closes. Adds `duration_s`. | AC-16.02 |
| `ticket.created` | Ticket submitted, when the warning route's webhook is on. Adds `ticket_number`, `title` and `ticket_url`; `text` reads `[TICKET] Payments #17: <title>`. | AC-30.03 |
| `test` | Test button | |

## 6.3 Outgoing email

| Event | Subject |
| --- | --- |
| Opened | `[CRITICAL] Payments / API health is down` |
| Resolved | `[RESOLVED] Payments / API health is up after 4m 12s` |
| Digest | `[Monita] Payments: 3 new tickets` |
| Invite | `You are invited to Monita` |

Plain text body with the cause and the incident link (AC-16.01). The digest goes to the project owner. It lists number, title and link of tickets created after the previous digest (a ticket-id watermark in `settings`) and still in Triage (AC-30.01), never the description, and is not sent when there are none (AC-30.02). It runs once a day at `MONITA_DIGEST_UTC_HOUR`.
