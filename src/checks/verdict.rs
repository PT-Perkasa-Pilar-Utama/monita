//! What one new check result means for a monitor that checks from several places (v1.1 Q26,
//! technical-specs 14.7). Pure: the caller reads each place's newest result.

/// The name the pages and causes give this server.
pub const THIS_SERVER: &str = "This server";

/// One place a monitor checks from, and its newest result: when, whether it passed, its error.
pub struct Place<'a> {
    pub name: &'a str,
    pub newest: Option<(i64, bool, Option<&'a str>)>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Verdict {
    Passed,
    Failed { cause: String },
}

/// Failed when at least `min(quorum, fresh)` of the places with a fresh result failed, and at
/// least one did. A result is fresh for two intervals and 30 seconds, so a silent location never
/// blocks an incident: the places that still answer decide.
pub fn judge(places: &[Place<'_>], interval_s: i64, quorum: i64, now: i64) -> Verdict {
    let oldest = now - (2 * interval_s + 30);
    let fresh: Vec<(&str, bool, Option<&str>)> = places
        .iter()
        .filter_map(|p| match p.newest {
            Some((at, ok, error)) if at >= oldest => Some((p.name, ok, error)),
            _ => None,
        })
        .collect();
    let failing: Vec<&(&str, bool, Option<&str>)> = fresh.iter().filter(|(_, ok, _)| !ok).collect();
    let needed = quorum.max(1).min(fresh.len() as i64);
    if failing.is_empty() || (failing.len() as i64) < needed {
        return Verdict::Passed;
    }
    let error = |e: Option<&str>| e.unwrap_or("Check failed").to_string();
    let cause = if places.len() == 1 {
        error(failing[0].2)
    } else {
        let each: Vec<String> = failing
            .iter()
            .map(|(name, _, e)| format!("{name}: {}", error(*e)))
            .collect();
        format!(
            "{} of {} places failed: {}",
            failing.len(),
            fresh.len(),
            each.join("; ")
        )
    };
    Verdict::Failed { cause }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 10_000;

    fn place(name: &'static str, result: Option<(i64, bool)>) -> Place<'static> {
        Place {
            name,
            newest: result.map(|(age, ok)| (NOW - age, ok, (!ok).then_some("HTTP 503"))),
        }
    }

    fn failed(v: &Verdict) -> bool {
        matches!(v, Verdict::Failed { .. })
    }

    // The table in technical-specs 14.7, row by row; interval 60 s, so fresh means at most 150 s old.
    #[test]
    fn the_spec_table_holds() {
        let one = [place(THIS_SERVER, Some((0, false)))];
        assert_eq!(
            judge(&one, 60, 1, NOW),
            Verdict::Failed {
                cause: "HTTP 503".into()
            },
            "as before 1.1"
        );

        let three = |a, b, c| {
            [
                place(THIS_SERVER, a),
                place("Jakarta", b),
                place("Tokyo", c),
            ]
        };
        assert!(!failed(&judge(
            &three(Some((0, false)), Some((5, true)), Some((9, true))),
            60,
            2,
            NOW
        )));
        assert!(failed(&judge(
            &three(Some((0, false)), Some((5, false)), Some((9, true))),
            60,
            2,
            NOW
        )));
        assert!(
            failed(&judge(
                &three(Some((0, false)), Some((151, true)), None),
                60,
                2,
                NOW
            )),
            "the others are stale or silent, so this server decides"
        );
        let two = [
            place(THIS_SERVER, Some((0, true))),
            place("Jakarta", Some((3, false))),
        ];
        assert!(!failed(&judge(&two, 60, 2, NOW)));
    }

    #[test]
    fn a_quorum_of_one_fails_when_any_fresh_place_fails() {
        let places = [
            place(THIS_SERVER, Some((0, true))),
            place("Jakarta", Some((0, false))),
        ];
        assert_eq!(
            judge(&places, 60, 1, NOW),
            Verdict::Failed {
                cause: "1 of 2 places failed: Jakarta: HTTP 503".into()
            }
        );
    }

    #[test]
    fn freshness_ends_two_intervals_and_30_seconds_back() {
        let at = |age| {
            [
                place(THIS_SERVER, Some((0, true))),
                place("Jakarta", Some((age, false))),
            ]
        };
        assert!(
            !failed(&judge(&at(150), 60, 2, NOW)),
            "one of two is short of a quorum of 2"
        );
        assert!(failed(&judge(&at(150), 60, 1, NOW)), "150 s is still fresh");
        assert!(!failed(&judge(&at(151), 60, 1, NOW)), "151 s is stale");
    }

    #[test]
    fn the_cause_names_each_failing_place() {
        let places = [
            Place {
                name: THIS_SERVER,
                newest: Some((NOW, false, Some("Timeout after 10000 ms"))),
            },
            Place {
                name: "Jakarta",
                newest: Some((NOW, false, None)),
            },
            Place {
                name: "Tokyo",
                newest: Some((NOW, true, None)),
            },
        ];
        assert_eq!(
            judge(&places, 60, 2, NOW),
            Verdict::Failed {
                cause: "2 of 3 places failed: This server: Timeout after 10000 ms; Jakarta: Check failed".into()
            }
        );
    }

    #[test]
    fn nothing_fresh_or_nothing_failing_passes() {
        assert_eq!(
            judge(&[place(THIS_SERVER, None)], 60, 1, NOW),
            Verdict::Passed
        );
        let ok = [
            place(THIS_SERVER, Some((0, true))),
            place("Jakarta", Some((0, true))),
        ];
        assert_eq!(judge(&ok, 60, 1, NOW), Verdict::Passed);
        assert_eq!(
            judge(&ok, 60, 0, NOW),
            Verdict::Passed,
            "a quorum below 1 reads as 1"
        );
    }
}
