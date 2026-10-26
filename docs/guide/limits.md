# Limits

No page or job in Monita grows without bound. A list you add to refuses one more item past its cap. Projects have no cap; the project list shows 20 per page.

| What | Cap |
| --- | --- |
| Projects per page of the project list | 20 |
| Accounts per instance | 500 |
| Monitors per project | 100 |
| Heartbeats per project | 100 |
| Live API keys per project | 20 |
| Secrets per project | 20, up to 4 KB each |
| Headers on a monitor | 20, up to 4 KB a value |
| Request body of a monitor | 64 KB |
| Alert cards in each section of a project (incidents, tickets), besides the owner's | 10 |
| Addresses per email alert card | 20 |
| Updates per incident | 50 |
| Open manual incidents per project | 20 |
| Tickets per board column | newest 25 shown, all counted; Show more loads the next 25 |
| Tickets per digest email | 50 listed, the rest counted |
| Checks and pings per page | 25, with an Older link |
| Releases per page of the changelog | 25 |
| Rows per page of the Incidents and Tickets lists across projects | 25 |
| Users per page of User management | 20 |
| Incidents per page of the public history | 25 |

## Rate limits

A super admin changes these under **Admin**, **Rate limits**. See [Admin and roles](admin.md#rate-limits). A request over a limit gets 429.

| What | Default |
| --- | --- |
| Checks at the same time | 50 |
| Wrong passwords for one account, then sign-in waits | 10 per 15 minutes |
| API calls per minute from one key | 120 |
| Pings per second for one heartbeat | 1 |
| Bug reports per user per day | 5 |
| Bug reports per project per day | 100 |

Request bodies are capped at 128 KiB (413), except the ping routes. See [Heartbeats](heartbeats.md#rules).

## Data kept

| Data | Kept |
| --- | --- |
| Raw checks and pings | 7 days |
| Daily summaries | indefinitely |
| Delivered alerts | 30 days |
| Backups | `MONITA_BACKUP_KEEP`, 7 by default |

A super admin sees these numbers and how many are in use under **Admin**, **Rate limits**, **Built-in maximums**. See [Admin and roles](admin.md#rate-limits).

The full table is in [spec 08 section 8.2](../technical-specs/08-non-functional-requirements.md).
