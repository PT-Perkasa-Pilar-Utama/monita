use super::*;

#[test]
fn a_name_limit_counts_characters_not_bytes() {
    let input = MonitorInput {
        name: "é".repeat(80),
        url: "https://x.co".into(),
        ..MonitorInput::defaults()
    };
    assert!(!input.validate().contains_key("name"));
}

#[test]
fn a_url_with_a_user_name_or_password_is_refused() {
    for url in ["https://bob:secret@x.co/health", "https://bob@x.co/health"] {
        let input = MonitorInput {
            name: "API".into(),
            url: url.into(),
            ..MonitorInput::defaults()
        };
        assert!(input.validate()["url"].contains("Sign in with"), "{url}");
    }
    let query = MonitorInput {
        name: "API".into(),
        url: "https://x.co/health?token=abc".into(),
        ..MonitorInput::defaults()
    };
    assert!(
        !query.validate().contains_key("url"),
        "a query string stays allowed"
    );
}

#[test]
fn a_tcp_target_needs_a_host_and_a_port() {
    for ok in [
        "db.internal:5432",
        "10.0.0.5:6379",
        "[2001:db8::1]:443",
        "redis_1:6379",
    ] {
        assert!(valid_tcp_target(ok), "{ok}");
    }
    for bad in [
        "db.internal",
        "db:0",
        "db:70000",
        "2001:db8::1:443",
        "[nope]:443",
        ":80",
        "http://db:80",
    ] {
        assert!(!valid_tcp_target(bad), "{bad}");
    }
}

#[test]
fn a_dns_host_is_a_name_or_an_address_without_a_port() {
    for ok in ["api.example.com", "localhost", "203.0.113.10", "::1"] {
        assert!(valid_host(ok), "{ok}");
    }
    for bad in ["", "-x.co", ".x.co", "x.co:53", "https://x.co", "a b"] {
        assert!(!valid_host(bad), "{bad}");
    }
}

#[test]
fn expected_status_accepts_classes_and_lists() {
    assert!(valid_expected_status("2xx"));
    assert!(valid_expected_status("200, 204,301"));
    assert!(!valid_expected_status("2x0"));
    assert!(!valid_expected_status("600"));
    assert!(!valid_expected_status("ok"));
    assert!(!valid_expected_status(""));
}

#[test]
fn defaults_pass_validation_once_name_and_url_are_set() {
    let mut m = MonitorInput::defaults();
    assert!(m.validate().contains_key("name"));
    m.name = "API health".into();
    m.url = "https://pay.internal/api/health".into();
    assert!(m.validate().is_empty());
    m.interval_s = 45;
    assert_eq!(
        m.validate().keys().copied().collect::<Vec<_>>(),
        vec!["interval_s"]
    );
}

#[test]
fn a_failure_count_runs_from_1_to_99() {
    let with = |n| MonitorInput {
        name: "API".into(),
        url: "https://x.co".into(),
        fails_to_open: n,
        ..MonitorInput::defaults()
    };
    assert!(errors(&with(1)).is_empty());
    assert!(errors(&with(37)).is_empty());
    assert_eq!(errors(&with(0)), ["fails_to_open"]);
    assert_eq!(errors(&with(100)), ["fails_to_open"]);
}

fn errors(m: &MonitorInput) -> Vec<&'static str> {
    m.validate().keys().copied().collect()
}

#[test]
fn each_sign_in_kind_needs_a_secret_and_its_own_field() {
    let base = MonitorInput {
        name: "API".into(),
        url: "https://x.co".into(),
        ..MonitorInput::defaults()
    };
    let with = |kind: &str| MonitorInput {
        auth_kind: kind.into(),
        ..base.clone()
    };
    assert!(errors(&with("")).is_empty(), "empty reads as none");
    assert_eq!(errors(&with("oauth")), ["auth_kind"]);
    assert_eq!(errors(&with("bearer")), ["auth_secret_id"]);
    let secret = |kind: &str| MonitorInput {
        auth_secret_id: Some(1),
        ..with(kind)
    };
    assert!(errors(&secret("bearer")).is_empty());
    assert_eq!(errors(&secret("basic")), ["auth_user"]);
    let colon = MonitorInput {
        auth_user: Some("a:b".into()),
        ..secret("basic")
    };
    assert_eq!(errors(&colon), ["auth_user"]);
    assert_eq!(errors(&secret("header")), ["auth_header"]);
    let host = MonitorInput {
        auth_header: Some("Host".into()),
        ..secret("header")
    };
    assert_eq!(errors(&host), ["auth_header"]);
}

#[test]
fn a_plain_header_may_not_repeat_the_one_authentication_sends() {
    let m = MonitorInput {
        name: "API".into(),
        url: "https://x.co".into(),
        auth_kind: "header".into(),
        auth_secret_id: Some(1),
        auth_header: Some("X-Api-Key".into()),
        headers: "x-api-key: plain".into(),
        ..MonitorInput::defaults()
    };
    assert_eq!(errors(&m), ["headers"]);
    let bearer = MonitorInput {
        auth_kind: "bearer".into(),
        headers: "Authorization: Bearer plain".into(),
        ..m.clone()
    };
    assert_eq!(errors(&bearer), ["headers"]);
    let none = MonitorInput {
        auth_kind: "none".into(),
        ..bearer
    };
    assert!(
        errors(&none).is_empty(),
        "no sign-in: a plain header is fine"
    );
}
