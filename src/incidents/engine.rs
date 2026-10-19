//! The pure incident rules. Spec: docs/technical-specs/12-incident-engine.md section 12.3.
//! No I/O: `observe` loads the state, calls `step`, and writes what it returns.

use super::Signal;

/// Confirmation for monitors: this many failed checks in a row open an incident (Q4). Each
/// monitor may choose another count from `FAILS_CHOICES`.
pub const FAILS_TO_OPEN: u32 = 2;
pub const FAILS_CHOICES: [u32; 4] = [1, 2, 3, 5];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SourceState {
    pub fail_streak: u32,
    /// An automatic incident is open for this source.
    pub open: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    Open,
    Resolve,
}

/// `fails_to_open` is the monitor's count of failed checks in a row; heartbeats ignore it.
pub fn step(s: SourceState, sig: &Signal, fails_to_open: u32) -> (SourceState, Option<Effect>) {
    match (s.open, sig) {
        (false, Signal::CheckFailed { .. }) => {
            let streak = s.fail_streak + 1;
            if streak >= fails_to_open.max(1) {
                (
                    SourceState {
                        fail_streak: streak,
                        open: true,
                    },
                    Some(Effect::Open),
                )
            } else {
                (
                    SourceState {
                        fail_streak: streak,
                        open: false,
                    },
                    None,
                )
            }
        }
        (false, Signal::HeartbeatMissed | Signal::FailPing { .. }) => (
            SourceState {
                fail_streak: 0,
                open: true,
            },
            Some(Effect::Open),
        ),
        (false, Signal::CheckOk | Signal::Ping) => (SourceState::default(), None),
        (true, Signal::CheckFailed { .. } | Signal::HeartbeatMissed | Signal::FailPing { .. }) => {
            (s, None)
        }
        (true, Signal::CheckOk | Signal::Ping) => (SourceState::default(), Some(Effect::Resolve)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn failed() -> Signal {
        Signal::CheckFailed {
            cause: "HTTP 503".into(),
        }
    }
    fn state(fail_streak: u32, open: bool) -> SourceState {
        SourceState { fail_streak, open }
    }

    /// Spec 12 section 12.3, one row per line.
    #[test]
    fn rules_table() {
        let fail_ping = Signal::FailPing {
            cause: "exit 1".into(),
        };
        let rows: [(SourceState, Signal, SourceState, Option<Effect>); 12] = [
            (state(0, false), failed(), state(1, false), None),
            (
                state(1, false),
                failed(),
                state(2, true),
                Some(Effect::Open),
            ),
            (state(1, false), Signal::CheckOk, state(0, false), None),
            (
                state(0, false),
                Signal::HeartbeatMissed,
                state(0, true),
                Some(Effect::Open),
            ),
            (
                state(0, false),
                fail_ping.clone(),
                state(0, true),
                Some(Effect::Open),
            ),
            (state(0, false), Signal::Ping, state(0, false), None),
            (state(2, true), failed(), state(2, true), None),
            (
                state(0, true),
                Signal::HeartbeatMissed,
                state(0, true),
                None,
            ),
            (state(0, true), fail_ping, state(0, true), None),
            (
                state(2, true),
                Signal::CheckOk,
                state(0, false),
                Some(Effect::Resolve),
            ),
            (
                state(0, true),
                Signal::Ping,
                state(0, false),
                Some(Effect::Resolve),
            ),
            (state(0, false), Signal::CheckOk, state(0, false), None),
        ];
        for (i, (from, sig, to, effect)) in rows.into_iter().enumerate() {
            assert_eq!(
                step(from, &sig, FAILS_TO_OPEN),
                (to, effect),
                "row {}",
                i + 1
            );
        }
    }

    #[test]
    fn one_blip_between_passes_never_opens() {
        let mut s = SourceState::default();
        for sig in [
            failed(),
            Signal::CheckOk,
            failed(),
            Signal::CheckOk,
            failed(),
        ] {
            let (next, effect) = step(s, &sig, FAILS_TO_OPEN);
            assert_eq!(effect, None);
            s = next;
        }
    }

    #[test]
    fn a_monitor_opens_after_its_own_count_of_failures() {
        let at_once = step(state(0, false), &failed(), 1);
        assert_eq!(at_once, (state(1, true), Some(Effect::Open)));
        let mut s = SourceState::default();
        for _ in 0..2 {
            let (next, effect) = step(s, &failed(), 3);
            assert_eq!(effect, None, "fewer than 3 failures");
            s = next;
        }
        assert_eq!(step(s, &failed(), 3), (state(3, true), Some(Effect::Open)));
    }
}
