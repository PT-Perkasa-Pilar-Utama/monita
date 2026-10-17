# 8. Non-Functional Requirements

| ID | Requirement | Target | Source |
| --- | --- | --- | --- |
| NFR-01 | Monitors per instance | 500 at a 60 s interval | Decision log Q12 sizing |
| NFR-02 | Heartbeats per instance | 1 000 | Assumed: two per project |
| NFR-03 | Scheduler lateness | p95 under 2 s from `next_check_at` | Loop ticks every 1 s |
| NFR-04 | Alert latency | Outbox row created within 1 s of the incident. Email or webhook sent within 60 s. | Q2 |
| NFR-05 | Page response | p95 under 200 ms server time for admin and status pages | Lightweight motto |
| NFR-06 | Memory | RSS under 100 MB at NFR-01 load | Lightweight motto |
| NFR-07 | Binary size | Under 30 MB | Lightweight motto |
| NFR-08 | Start time | Serving in under 2 s with a 1 GB database | Fast restart under systemd |
| NFR-09 | Database size | Under 2 GB at NFR-01 with 7-day raw retention | 720 k rows a day, about 60 bytes per row, plus indexes |
| NFR-10 | Availability | 99.9 % per instance. The peer instance detects an outage within period + grace of its heartbeat. | Q15, T2 |
| NFR-11 | Data loss on crash | No lost alerts (outbox). At most the last check result per monitor. | Reliability motto |
| NFR-12 | Backup | Daily, 7 kept. RPO 24 h. | US-33 |

These numbers are the targets that [12](12-incident-engine.md) and [13](13-check-scheduler.md) refer to.

## 8.2 Limits

No list is unbounded. A list people add to is refused at insert past its cap (422 with a plain message), and its read uses the same number as its `LIMIT`. Source: `src/platform/caps.rs`; tests: `tests/limits.rs`.

| What | Cap | At the cap |
| --- | --- | --- |
| Projects per instance | 200 | New project refused |
| Accounts per instance | 500 | Invite and `admin create` refused |
| Monitors per project | 100 | New monitor refused |
| Heartbeats per project | 100 | New heartbeat refused |
| Live API keys per project | 20 | New key refused; settings also shows the newest 20 revoked |
| Addresses per alert route | 20 | Route not saved |
| Status updates per incident | 50 | Update refused |
| Open manual incidents per project | 20 | New manual incident refused |
| Tickets per board column | 200 shown | The newest show; the header counts all |
| Tickets per digest email | 50 listed | The email says how many more wait |
| Updates per incident on the status page | 10 | The newest show |
| Incidents on the status page | 200 | The newest show |
| Automatic-incident spans read per window | 10 000 | The oldest are left out of the uptime figure |
| Pings per heartbeat page, checks per monitor page | 100 | An "Older" link pages back |
| Missed heartbeats per write | 50 | The sweep loops |
| Rows per job batch (rollup, outbox and session purge) | 500 | The job loops |
