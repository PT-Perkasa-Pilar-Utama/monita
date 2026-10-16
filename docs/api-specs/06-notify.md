# 6. Notify

Fields: [../technical-specs/06-data-model.md section 6.3.6](../technical-specs/06-data-model.md#636-alert_routes).

## 6.1 HTML routes

| Method | Path | Access | Input | Behavior | AC |
| --- | --- | --- | --- | --- | --- |
| GET | `/projects/{slug}/alerts` | owner, super_admin | | One block per severity: emails, webhook URL, use webhook | AC-15.01, AC-15.02 |
| POST | `/projects/{slug}/alerts/{severity}` | owner, super_admin | `emails` (one per line), `webhook_url`, `use_webhook` | Validate each email and the URL. Save. | AC-15.01 |
| POST | `/projects/{slug}/alerts/{severity}/test` | owner, super_admin | | Queue one test email and webhook | |

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
| `ticket.created` | Ticket submitted. Warning route only. Adds `ticket_number` and `title`. | AC-30.03 |
| `test` | Test button | |

## 6.3 Outgoing email

| Event | Subject |
| --- | --- |
| Opened | `[CRITICAL] Payments / API health is down` |
| Resolved | `[RESOLVED] Payments / API health is up after 4m 12s` |
| Digest | `[Monita] Payments: 3 new tickets` |
| Invite | `You are invited to Monita` |

Plain text body with the cause and the incident link (AC-16.01). The digest lists ticket number, title and link (AC-30.01), and is not sent when there are no new tickets (AC-30.02).
