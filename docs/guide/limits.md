# Limits

No page or job in Monita grows without bound. A list you add to refuses one more item past its cap. Projects have no cap; the project list shows 20 per page.

| What | Cap |
| --- | --- |
| Projects per page of the project list | 20 |
| Accounts per instance | 500 |
| Monitors per project | 100 |
| Heartbeats per project | 100 |
| Live API keys per project | 20 |
| Addresses per alert route | 20 |
| Updates per incident | 50 |
| Open manual incidents per project | 20 |
| Tickets per board column | newest 200 shown, all counted |
| Tickets per digest email | 50 listed, the rest counted |
| Checks and pings per page | 25, with an Older link |

## Data kept

| Data | Kept |
| --- | --- |
| Raw checks and pings | 7 days |
| Daily summaries | indefinitely |
| Delivered alerts | 30 days |
| Backups | `MONITA_BACKUP_KEEP`, 7 by default |

The full table is in [spec 08 section 8.2](../technical-specs/08-non-functional-requirements.md).
