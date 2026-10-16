# 13. Check Scheduler

One loop drives every timed job in Monita: monitor checks, heartbeat deadlines, the daily rollup, retention, backup, digest and the outbox. Other docs and task cards cite this file and do not restate it.

## 13.1 Decision matrix

| Concern | Decision | Rationale |
| --- | --- | --- |
| Driver | One tokio loop that ticks every 1 s and reads due work from the database | T3. Edits apply on the next tick. A restart resumes. |
| Monitor due | `next_check_at <= now` | Indexed column |
| Claim | Set `next_check_at = now + interval_s` before the probe starts | A slow probe is never picked twice. |
| Concurrency | `tokio::sync::Semaphore` with `settings.check_concurrency` permits (50), re-read every tick | Q10 |
| Batch size | `LIMIT available_permits` | The loop never queues more than it can run. |
| New monitor offset | `next_check_at = now + rand(0..interval_s)` | Q10. Spreads load. |
| Interval edit | `next_check_at = now + rand(0..interval_s)` | Same spreading |
| Heartbeat due | `deadline_at <= now`. The loop sends `HeartbeatMissed`, then sets `deadline_at = NULL`. | One miss signal per outage |
| Ping | `deadline_at = now + period_s + grace_s` | Q7 |
| New heartbeat | `deadline_at = created_at + period_s + grace_s` | A job that never pings still alerts. |
| Daily jobs | Run when the UTC hour has passed and `settings.last_<job>_day` is not today | One loop, no cron crate |
| Liveness | An `AtomicI64` holds the last tick time. `/healthz` reads it. | T5 |

## 13.2 Tick

```text
every 1 s:
  last_tick = now
  permits = semaphore.available_permits()
  for m in SELECT ... FROM monitors WHERE next_check_at <= now ORDER BY next_check_at LIMIT permits:
      UPDATE monitors SET next_check_at = now + interval_s WHERE id = m.id
      spawn with permit:
          r = probe(m)
          tx: INSERT check_results; ev = incidents::observe(Monitor(m.id), signal(r)); notify::send(ev)
  for h in SELECT ... FROM heartbeats WHERE deadline_at <= now:
      tx: UPDATE heartbeats SET deadline_at = NULL; ev = observe(Heartbeat(h.id), HeartbeatMissed); send(ev)
  outbox::deliver_due()          (spawned, one at a time)
  daily jobs, each once per UTC day:
      00:10  rollup yesterday into daily_summaries, then delete check_results and pings older than 7 days,
             outbox rows older than 30 days, expired sessions and invites
      01:00  backup
      MONITA_DIGEST_UTC_HOUR  ticket digest
```

## 13.3 Performance targets

| Target | Value | Source |
| --- | --- | --- |
| Lateness p95 | under 2 s | NFR-03 |
| Tick cost with nothing due | under 2 ms | Two indexed queries |
| Throughput | 500 checks/min with 50 slots and 10 s timeouts | NFR-01: worst case 50 slots × 6 checks/min = 300 timed-out checks/min, so a mass outage delays checks. Accepted. |

## 13.4 Constraints

- Probes never hold a DB transaction open while waiting on the network.
- One write transaction per check result. Turso runs one writer at a time, so writes are short.
- On shutdown (SIGTERM) the loop stops claiming, waits up to 10 s for running probes, then exits.

## 13.5 Contract it produces

```text
src/checks/scheduler.rs   pub fn spawn(state: AppState) -> JoinHandle<()>
src/checks/retention.rs   pub async fn rollup_and_sweep(conn, day) -> Result<()>
```

## 13.6 What this does NOT do

| Non-goal | Where it lives |
| --- | --- |
| Decide incidents | [12](12-incident-engine.md) |
| Deliver email | `notify::outbox` (the loop only triggers it) |
| Run on more than one instance against one DB | Out of scope. See [02 section 2.5](02-system-architecture.md#25-replication-model). |

## 13.7 Open follow-ups

- Check concurrency above 50, if a mass outage delays checks too much. Trigger: lateness p95 over 10 s during an outage.
- Resolved 2026-10-16: writes go through one writer connection behind a mutex ([05 section 5.8.5](05-module-definitions.md#585-database-access)). Revisit only if write lock wait p95 exceeds 50 ms.
