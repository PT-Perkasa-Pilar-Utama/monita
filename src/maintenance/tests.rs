use super::form::{LONGEST_S, MESSAGE_MAX};
use super::*;

const NOW: i64 = 1_000_000;

fn names() -> Vec<(SourceKind, i64, String)> {
    vec![
        (SourceKind::Monitor, 1, "api".into()),
        (SourceKind::Heartbeat, 2, "nightly".into()),
    ]
}

fn form() -> WindowInput {
    WindowInput {
        title: "Database upgrade".into(),
        starts_at: Some(NOW + 60),
        ends_at: Some(NOW + 3600),
        items: vec!["monitor:1".into(), "monitor:9".into()],
        ..WindowInput::default()
    }
}

fn fields(f: &WindowInput) -> Vec<&'static str> {
    match validate(f, &names(), NOW) {
        Err(MaintenanceError::Invalid(bad)) => bad.into_iter().map(|b| b.0).collect(),
        other => panic!("expected field errors, got {other:?}"),
    }
}

#[test]
fn the_state_follows_from_the_times() {
    assert_eq!(state(10, 20, 9), State::Scheduled);
    assert_eq!(state(10, 20, 10), State::Active);
    assert_eq!(state(10, 20, 19), State::Active);
    assert_eq!(state(10, 20, 20), State::Ended);
}

#[test]
fn a_good_form_keeps_only_the_projects_items_and_a_past_start_starts_now() {
    let (_, _, s, _, items) = validate(&form(), &names(), NOW).unwrap();
    assert_eq!(s, NOW + 60);
    assert_eq!(items.len(), 1, "monitor 9 is not in the project");
    let past = WindowInput {
        starts_at: Some(NOW - 600),
        ..form()
    };
    assert_eq!(validate(&past, &names(), NOW).unwrap().2, NOW);
}

#[test]
fn a_bad_form_names_every_bad_field() {
    let empty = WindowInput::default();
    assert_eq!(fields(&empty), ["title", "starts_at", "ends_at", "items"]);
    let ended = WindowInput {
        ends_at: Some(NOW),
        ..form()
    };
    assert_eq!(fields(&ended), ["ends_at"]);
    let backwards = WindowInput {
        starts_at: Some(NOW + 7200),
        ..form()
    };
    assert_eq!(fields(&backwards), ["ends_at"]);
    let long = WindowInput {
        ends_at: Some(NOW + 60 + LONGEST_S + 1),
        ..form()
    };
    assert_eq!(fields(&long), ["ends_at"]);
    let loud = WindowInput {
        message: "x".repeat(MESSAGE_MAX + 1),
        ..form()
    };
    assert_eq!(fields(&loud), ["message"]);
}

#[test]
fn an_active_edit_checks_its_end_against_the_start_it_has() {
    let start = NOW - 600;
    let ok = WindowInput {
        ends_at: Some(NOW + 600),
        ..form()
    };
    assert_eq!(validate_active(&ok, start, NOW).unwrap().2, NOW + 600);
    let bad = |f: &WindowInput| match validate_active(f, start, NOW) {
        Err(MaintenanceError::Invalid(b)) => b.into_iter().map(|b| b.0).collect::<Vec<_>>(),
        other => panic!("expected field errors, got {other:?}"),
    };
    let past = WindowInput {
        ends_at: Some(NOW),
        ..form()
    };
    assert_eq!(bad(&past), ["ends_at"]);
    let long = WindowInput {
        ends_at: Some(start + LONGEST_S + 1),
        ..form()
    };
    assert_eq!(
        bad(&long),
        ["ends_at"],
        "30 days count from the start it has"
    );
    let blank = WindowInput {
        title: " ".into(),
        items: Vec::new(),
        ..ok
    };
    assert_eq!(bad(&blank), ["title"], "the items are not asked");
}
