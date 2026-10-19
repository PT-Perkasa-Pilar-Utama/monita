# 12. Incident Engine

The incident engine turns check signals into incidents and events. It applies to monitors, heartbeats and manual incidents. Other docs and task cards cite this file and do not restate its rules.

## 12.1 Decision matrix

| Concern | Decision | Rationale |
| --- | --- | --- |
| Entry point | `incidents::observe(source, signal)` is the only door | One place for the rules (T4) |
| Confirmation | A monitor needs `fails_to_open` failed checks in a row. Each monitor sets it from 1 to 99; the default is 2. | Q4 set 2 as the rule. Each monitor now chooses, and 2 is the default. One blip is not an outage. |
| Heartbeat miss | Opens at once when `deadline_at` passes | The grace time is already the confirmation (Q7). |
| Fail ping | Opens at once | Q7. The job reports its own failure. |
| Close | The first passing check or ping closes an automatic incident. A TL may also close it by hand (below). | Q4. Downtime stays exact. |
| Hand resolve | A TL posts Resolved on an open automatic incident. It closes, the resolved alert says "marked resolved by a tech lead" (`Ended::ByHand`), and the source starts over: streak, open incident and `down_since` are cleared. A heartbeat gets `deadline_at = now + period + grace` from `checks::rearm_heartbeat`, so a silent job opens the next incident one period plus grace later. The TL's update is the close; Monita writes no "Recovered" entry. | The job may be retired on purpose. A monitor needs `fails_to_open` new failures to open again. |
| Alert switch | `alert` off on a source: the engine runs as usual and `observe` returns no event, so nobody is told. A hand resolve sends nothing either. | The TL tracks a source but wants no noise. |
| Incident switch | `auto_incident` off on a source: no incident opens. A failure sends `Event::Down` (`source.down`, `[DOWN]`) and recovery sends `Event::Up` (`source.up`, `[UP]`). `incident_sources.down_since` holds when it went down. The team's pages show Down (`team_states_for`). The status page shows nothing. | The TL wants the alert without a public incident. |
| Manual incidents | Never closed by signals. The TL posts Resolved. | Q5 |
| TL labels | An open incident takes all four labels. Resolved closes a manual incident, and closes an automatic one by hand. A closed incident takes only Resolved, as a note (AC-19.03). | The form dims a label the incident cannot take and says why. |
| One open auto incident per source | `incident_sources.open_incident_id` | No duplicate incidents for one outage |
| System timeline entries | On open: label Investigating, message "Detected: <cause>". On close: label Resolved, message "Recovered after <duration>". Author NULL. | The timeline has a start and an end without TL action. |
| Pure core | `engine::step(state, signal, fails_to_open) -> (state, Option<Effect>)` with no I/O | Unit tests cover every rule without a database. |
| Atomicity | `observe` reads state, applies effects and writes in one transaction | A crash cannot leave an open incident with no event. |
| Alerts | `observe` returns `Event::Opened`, `Resolved`, `Down` or `Up`. The caller passes it to `notify::send` in the same transaction. | Outbox rows exist exactly when the incident row does. |

## 12.2 Interface

```rust
pub struct Source {
    kind: SourceKind, id: i64, project_id: i64, name: String, severity: String,
    alert: bool,           // its failures alert
    auto_incident: bool,   // its failures open an incident
    fails_to_open: u32,    // monitors only; a heartbeat ignores it
}

pub enum Signal {
    CheckOk,
    CheckFailed { cause: String },  // monitor
    HeartbeatMissed,                // deadline passed
    FailPing { cause: String },     // /ping/{token}/fail
    Ping,                           // /ping/{token}
}

pub enum Event {
    Opened { incident_id: i64, project_id: i64, source_name: String, title: String, severity: String, cause: String },
    Resolved { incident_id: i64, project_id: i64, source_name: String, severity: String, duration_s: i64, ended: Ended },
    /// Down with `auto_incident` off: the alert alone tells the team.
    Down { project_id: i64, kind: SourceKind, source_id: i64, source_name: String, title: String, severity: String, cause: String },
    /// That source works again.
    Up { project_id: i64, kind: SourceKind, source_id: i64, source_name: String, severity: String, duration_s: i64 },
}

/// Why an automatic incident ended. Only `Recovered` means the source works again.
pub enum Ended { Recovered, Deleted, ByHand }

/// Applies one signal. Returns the event to send, if any.
/// Errors only on database failure.
/// `Source` is filled by the checks module, so this module never reads checks tables.
/// With `alert` off the event is dropped before it returns.
pub async fn observe(c: &Connection, source: &Source, signal: &Signal, now: i64) -> Result<Option<Event>>;
```

Pure core:

```rust
/// `open`: an incident is open, or the source is down with its incident switch off.
pub struct SourceState { pub fail_streak: u32, pub open: bool }
pub enum Effect { Open, Resolve }

/// `fails_to_open` is the monitor's count (1 to 99, `FAILS_RANGE`). Heartbeats ignore it.
pub fn step(s: SourceState, sig: &Signal, fails_to_open: u32) -> (SourceState, Option<Effect>);
```

## 12.3 Rules of `step`

N is the source's `fails_to_open`. With N = 1 the first failed check opens the incident. A streak that reached N stays at N while the incident is open.

| Current | Signal | New streak | Open after | Effect |
| --- | --- | --- | --- | --- |
| closed, streak below N-1 | CheckFailed | streak + 1 | no | none |
| closed, streak N-1 | CheckFailed | N | yes | Open |
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
  mod.rs        observe, source_deleted, states_for, team_states_for, across, item_names, Event, Ended, Source, Signal
  engine.rs     SourceState, Effect, step, FAILS_RANGE   (pure, 100 % branch-tested)
  updates.rs    post_update, allowed_labels, the hand resolve
  manual.rs     open_manual: the tech lead's reported incident
  history.rs    downtime per day, the public incident history
  availability.rs  uptime figures from automatic incidents
  store/        SQL for incidents, incident_items, incident_sources, status_updates; mod, reads, across
  api.rs        /api/ui JSON: list, detail, post update, open manual
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

- Resolved 2026-10-19: the confirmation count is per monitor (migrations 0014 and 0018). 2 stays the default.
- A "degraded" state for slow responses. Revisit when a latency threshold is requested.
