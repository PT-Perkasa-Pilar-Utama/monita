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
