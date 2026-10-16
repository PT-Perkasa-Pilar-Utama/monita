# 12. Incident Engine

The incident engine turns check signals into incidents and events. It applies to monitors, heartbeats and manual incidents. Other docs and task cards cite this file and do not restate its rules.

## 12.1 Decision matrix

| Concern | Decision | Rationale |
| --- | --- | --- |
| Entry point | `incidents::observe(source, signal)` is the only door | One place for the rules (T4) |
| Confirmation | A monitor needs 2 failed checks in a row | Q4. One blip is not an outage. |
| Heartbeat miss | Opens at once when `deadline_at` passes | The grace time is already the confirmation (Q7). |
| Fail ping | Opens at once | Q7. The job reports its own failure. |
| Close | The first passing check or ping closes an automatic incident | Q4. Downtime stays exact. |
| Manual incidents | Never closed by signals. The TL posts Resolved. | Q5 |
| One open auto incident per source | `incident_sources.open_incident_id` | No duplicate incidents for one outage |
| System timeline entries | On open: label Investigating, message "Detected: <cause>". On close: label Resolved, message "Recovered after <duration>". Author NULL. | The timeline has a start and an end without TL action. |
| Pure core | `engine::step(state, signal) -> (state, Vec<Effect>)` with no I/O | Unit tests cover every rule without a database. |
| Atomicity | `observe` reads state, applies effects and writes in one transaction | A crash cannot leave an open incident with no event. |
| Alerts | `observe` returns `Event::Opened` or `Event::Resolved`. The caller passes it to `notify::send` in the same transaction. | Outbox rows exist exactly when the incident row does. |

## 12.2 Interface

```rust
pub enum Source { Monitor(i64), Heartbeat(i64) }

pub enum Signal {
    CheckOk,
    CheckFailed { cause: String },  // monitor
    HeartbeatMissed,                // deadline passed
    FailPing { cause: String },     // /ping/{token}/fail
    Ping,                           // /ping/{token}
}

pub enum Event {
    Opened { incident_id: i64, project_id: i64, source: Source, severity: Severity, cause: String },
    Resolved { incident_id: i64, project_id: i64, source: Source, severity: Severity, duration_s: i64 },
}

/// Applies one signal. Returns the event to send, if any.
/// Errors only on database failure.
/// `Source` carries kind, id, project_id, name and severity, filled by the checks module,
/// so this module never reads checks tables.
pub async fn observe(c: &Connection, source: &Source, signal: &Signal, now: i64) -> Result<Option<Event>>;
```

Pure core:

```rust
pub struct SourceState { pub fail_streak: u32, pub open: bool }
pub enum Effect { Open { cause: String }, Resolve }

pub fn step(s: SourceState, sig: &Signal) -> (SourceState, Option<Effect>);
```

## 12.3 Rules of `step`

| Current | Signal | New streak | Open after | Effect |
| --- | --- | --- | --- | --- |
| closed, streak 0 | CheckFailed | 1 | no | none |
| closed, streak 1 | CheckFailed | 2 | yes | Open |
| closed, any | CheckOk | 0 | no | none |
| closed | HeartbeatMissed | 0 | yes | Open |
| closed | FailPing | 0 | yes | Open |
| closed | Ping | 0 | no | none |
| open | CheckFailed, HeartbeatMissed, FailPing | unchanged | yes | none |
| open | CheckOk, Ping | 0 | no | Resolve |

## 12.4 Performance targets

| Target | Value | Source |
| --- | --- | --- |
| `observe` time | under 5 ms per call | NFR-03: 500 calls a minute must not delay the loop |
| Event to outbox row | same transaction | NFR-04 |

## 12.5 Contract it produces

```text
src/incidents/
  mod.rs      observe, open_manual, post_update, current_state, Event, Source, Signal
  engine.rs   SourceState, Effect, step            (pure, 100 % branch-tested)
  store.rs    SQL for incidents, incident_items, incident_sources, status_updates
  routes.rs   TL pages: list, detail, post update, open manual
```

## 12.6 What this does NOT do

| Non-goal | Where it lives |
| --- | --- |
| Decide who gets told | `notify` (alert routes) |
| Run checks or detect missed deadlines | `checks` scheduler ([13](13-check-scheduler.md)) |
| Escalate or repeat alerts | Out of scope for v1 |
| Acknowledge | Out of scope. One owner, no escalation. |

## 12.7 Cross-references

| Concern | Source |
| --- | --- |
| Incident state machine | [06 section 6.4.1](06-data-model.md#641-incident) |
| Alert routing | [05 section 5.5](05-module-definitions.md#55-notify) |
| Acceptance criteria | AC-17.01 to AC-20.02 |

## 12.8 Open follow-ups

- A per-monitor confirmation count. Revisit if TLs report noise or slow detection with 2.
- A "degraded" state for slow responses. Revisit when a latency threshold is requested.
