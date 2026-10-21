//! v1.1 Q19 (#12): two-factor sign-in with time codes and recovery codes. The tests compute
//! codes with the same RFC 6238 function an authenticator app uses.

mod common;

use common::{PASSWORD, TestApp, cookie_of, json_of};
use monita::{accounts::totp::code_at, platform::now};
use serde_json::{Value, json};

/// Turns two-factor on for the session's user; returns the secret, the recovery codes, and the
/// time whose code confirmed it.
async fn turn_on(app: &TestApp, cookie: &str) -> (String, Vec<String>, i64) {
    let started = json_of(
        app.api_post("/api/ui/me/two-factor", json!({}), Some(cookie))
            .await,
    )
    .await;
    let secret = started["data"]["secret"].as_str().unwrap().to_string();
    assert!(
        started["data"]["link"]
            .as_str()
            .unwrap()
            .starts_with("otpauth://totp/Monita:"),
        "{started}"
    );
    let qr = started["data"]["qr"].as_str().unwrap();
    assert!(
        qr.contains("<svg") && qr.contains("<path"),
        "the setup step draws the link as a QR code"
    );
    let wrong = app
        .api_post(
            "/api/ui/me/two-factor/confirm",
            json!({ "code": "000000" }),
            Some(cookie),
        )
        .await;
    assert_eq!(wrong.status(), 422);
    assert!(json_of(wrong).await["error"]["fields"]["code"].is_string());
    let at = now();
    let ok = app
        .api_post(
            "/api/ui/me/two-factor/confirm",
            json!({ "code": code_at(&secret, at) }),
            Some(cookie),
        )
        .await;
    assert_eq!(ok.status(), 200);
    let codes: Vec<String> = json_of(ok).await["data"]["recovery_codes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c.as_str().unwrap().to_string())
        .collect();
    (secret, codes, at)
}

/// The password step: the challenge token, and no session cookie yet.
async fn password_step(app: &TestApp, email: &str) -> String {
    let res = app
        .api_post(
            "/api/ui/login",
            json!({ "email": email, "password": PASSWORD }),
            None,
        )
        .await;
    assert_eq!(res.status(), 200);
    assert!(
        res.headers().get("set-cookie").is_none(),
        "no session before the code"
    );
    let body: Value = json_of(res).await;
    assert_eq!(body["data"]["two_factor"], true);
    body["data"]["challenge"].as_str().unwrap().to_string()
}

async fn code_step(app: &TestApp, challenge: &str, code: &str) -> reqwest::Response {
    app.api_post(
        "/api/ui/login/code",
        json!({ "challenge": challenge, "code": code }),
        None,
    )
    .await
}

#[tokio::test]
async fn with_two_factor_on_sign_in_needs_a_code_that_works_once() {
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    let me = async || {
        json_of(app.get("/api/ui/me", Some(&tl)).await).await["data"]["two_factor"].clone()
    };
    assert_eq!(me().await, false, "pages note two-factor while it is off");
    let (secret, codes, at) = turn_on(&app, &tl).await;
    assert_eq!(me().await, true);
    assert_eq!(codes.len(), 10);
    let status = json_of(app.api_get("/api/ui/me/two-factor", Some(&tl)).await).await;
    assert_eq!(status["data"], json!({ "on": true, "codes_left": 10 }));

    let challenge = password_step(&app, "rina@x.co").await;
    assert_eq!(code_step(&app, &challenge, "000000").await.status(), 422);
    let used = code_at(&secret, at);
    assert_eq!(
        code_step(&app, &challenge, &used).await.status(),
        422,
        "no code works twice"
    );
    let res = code_step(&app, &challenge, &code_at(&secret, at + 30)).await;
    assert_eq!(res.status(), 204);
    let session = cookie_of(&res);
    assert_eq!(
        app.api_get("/api/ui/me", Some(&session)).await.status(),
        200
    );
    assert_eq!(
        code_step(&app, &challenge, &code_at(&secret, now() - 30))
            .await
            .status(),
        410,
        "a used challenge is gone"
    );
}

#[tokio::test]
async fn a_recovery_code_works_once_and_five_wrong_codes_end_the_sign_in() {
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    let (_, codes, _) = turn_on(&app, &tl).await;

    let challenge = password_step(&app, "rina@x.co").await;
    let typed = codes[0].to_uppercase().replace('-', " ");
    assert_eq!(code_step(&app, &challenge, &typed).await.status(), 204);
    let again = password_step(&app, "rina@x.co").await;
    assert_eq!(
        code_step(&app, &again, &codes[0]).await.status(),
        422,
        "used up"
    );
    let status = json_of(app.api_get("/api/ui/me/two-factor", Some(&tl)).await).await;
    assert_eq!(status["data"]["codes_left"], 9);

    for _ in 0..3 {
        assert_eq!(code_step(&app, &again, "111111").await.status(), 422);
    }
    assert_eq!(
        code_step(&app, &again, "111111").await.status(),
        410,
        "the fifth wrong code"
    );
    assert_eq!(code_step(&app, &again, &codes[1]).await.status(), 410);
}

#[tokio::test]
async fn the_user_with_a_code_or_a_super_admin_turns_it_off() {
    let app = TestApp::spawn("test").await;
    let admin = app.super_admin("admin@x.co").await;
    let tl = app.tl("rina@x.co").await;
    let (secret, _, _) = turn_on(&app, &tl).await;

    let users = json_of(app.api_get("/api/ui/admin/users", Some(&admin)).await).await;
    let rina = users["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["email"] == "rina@x.co")
        .unwrap();
    assert_eq!(rina["two_factor"], true);
    let id = rina["id"].as_i64().unwrap();

    let off = |cookie: String, path: String| {
        let app = &app;
        async move { app.api_post(&path, json!({}), Some(&cookie)).await.status() }
    };
    assert_eq!(
        off(
            tl.clone(),
            format!("/api/ui/admin/users/{id}/two-factor/off")
        )
        .await,
        403
    );
    assert_eq!(
        off(
            admin.clone(),
            format!("/api/ui/admin/users/{id}/two-factor/off")
        )
        .await,
        204
    );
    // The password alone signs in again: `login` asserts 204 and a cookie.
    app.login("rina@x.co", PASSWORD).await;
    let admin_id = users["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["email"] == "admin@x.co")
        .unwrap()["id"]
        .as_i64()
        .unwrap();
    assert_eq!(
        off(
            admin.clone(),
            format!("/api/ui/admin/users/{admin_id}/two-factor/off")
        )
        .await,
        422,
        "not on your own account here"
    );

    let (secret2, _, at2) = turn_on(&app, &tl).await;
    assert_ne!(secret, secret2, "a new secret each time");
    let wrong = app
        .api_post(
            "/api/ui/me/two-factor/off",
            json!({ "code": "123" }),
            Some(&tl),
        )
        .await;
    assert_eq!(wrong.status(), 422);
    let right = app
        .api_post(
            "/api/ui/me/two-factor/off",
            json!({ "code": code_at(&secret2, at2 + 30) }),
            Some(&tl),
        )
        .await;
    assert_eq!(right.status(), 204);
    let status = json_of(app.api_get("/api/ui/me/two-factor", Some(&tl)).await).await;
    assert_eq!(status["data"], json!({ "on": false, "codes_left": 0 }));
    let again = app
        .api_post(
            "/api/ui/me/two-factor/off",
            json!({ "code": "123456" }),
            Some(&tl),
        )
        .await;
    assert_eq!(again.status(), 422, "already off");
}

#[tokio::test]
async fn a_supervisor_secures_their_own_account_and_the_cli_reset_clears_a_super_admins() {
    let app = TestApp::spawn("test").await;
    let admin = app.super_admin("admin@x.co").await;
    let sup = app.tl("sup@x.co").await;
    let users = json_of(app.api_get("/api/ui/admin/users", Some(&admin)).await).await;
    let sup_id = users["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["email"] == "sup@x.co")
        .unwrap()["id"]
        .clone();
    let role = app
        .api_send(
            reqwest::Method::PUT,
            &format!("/api/ui/admin/users/{sup_id}/role"),
            json!({ "role": "supervisor" }),
            Some(&admin),
        )
        .await;
    assert_eq!(role.status(), 204);
    turn_on(&app, &sup).await;
    assert_eq!(
        app.api_post(
            "/api/ui/projects",
            json!({ "name": "X", "slug": "x" }),
            Some(&sup)
        )
        .await
        .status(),
        403,
        "still read-only everywhere else"
    );

    turn_on(&app, &admin).await;
    password_step(&app, "admin@x.co").await;
    monita::accounts::reset_super_admin_password(&app.state.db, "admin@x.co", PASSWORD)
        .await
        .unwrap();
    app.login("admin@x.co", PASSWORD).await;
}

#[tokio::test]
async fn a_reset_sign_in_starts_over_without_two_factor() {
    let app = TestApp::spawn("test").await;
    let admin = app.super_admin("admin@x.co").await;
    let tl = app.tl("rina@x.co").await;
    turn_on(&app, &tl).await;
    let users = json_of(app.api_get("/api/ui/admin/users", Some(&admin)).await).await;
    let id = users["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["email"] == "rina@x.co")
        .unwrap()["id"]
        .clone();
    let invite = app
        .api_post(
            &format!("/api/ui/admin/users/{id}/invite"),
            json!({}),
            Some(&admin),
        )
        .await;
    let link = json_of(invite).await["data"]["link"]
        .as_str()
        .unwrap()
        .to_string();
    let fresh = app
        .accept(link.strip_prefix(common::BASE_URL).unwrap(), PASSWORD)
        .await;
    let status = json_of(app.api_get("/api/ui/me/two-factor", Some(&fresh)).await).await;
    assert_eq!(status["data"]["on"], false);
}

/// The fail-safe: `monita admin reset-sign-in` for a user who lost the app and the codes.
#[tokio::test]
async fn the_cli_reset_sign_in_turns_two_factor_off_and_emails_a_new_link() {
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    turn_on(&app, &tl).await;
    let err =
        monita::accounts::reset_sign_in_by_email(&app.state.db, common::BASE_URL, "nobody@x.co")
            .await
            .unwrap_err();
    assert_eq!(err.to_string(), "No active user has this email.");
    let link =
        monita::accounts::reset_sign_in_by_email(&app.state.db, common::BASE_URL, " Rina@X.co ")
            .await
            .unwrap();
    let old = app
        .api_post(
            "/api/ui/login",
            json!({ "email": "rina@x.co", "password": PASSWORD }),
            None,
        )
        .await;
    assert_eq!(old.status(), 422, "the old password stops");
    monita::notify::deliver_due(&app.state).await.unwrap();
    assert!(
        app.state
            .mailer
            .sent()
            .iter()
            .any(|m| m.to == "rina@x.co" && m.body.contains(&link)),
        "the link goes by email"
    );
    let fresh = app
        .accept(link.strip_prefix(common::BASE_URL).unwrap(), PASSWORD)
        .await;
    let status = json_of(app.api_get("/api/ui/me/two-factor", Some(&fresh)).await).await;
    assert_eq!(status["data"]["on"], false);
}

#[tokio::test]
async fn ten_wrong_codes_lock_the_account_s_codes_across_new_challenges() {
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    let (secret, _, at) = turn_on(&app, &tl).await;
    // The budget's window is a fixed 15 minutes; start well inside one, so the test cannot
    // straddle a reset.
    let into = now().rem_euclid(900);
    if into > 885 {
        tokio::time::sleep(std::time::Duration::from_secs((901 - into) as u64)).await;
    }
    // The password makes a new challenge each time; the budget still counts per account.
    for _ in 0..2 {
        let challenge = password_step(&app, "rina@x.co").await;
        for _ in 0..4 {
            assert_eq!(code_step(&app, &challenge, "000000").await.status(), 422);
        }
        assert_eq!(code_step(&app, &challenge, "000000").await.status(), 410);
    }
    let challenge = password_step(&app, "rina@x.co").await;
    let right = code_at(&secret, at + 30);
    let refused = code_step(&app, &challenge, &right).await;
    assert_eq!(refused.status(), 429, "even the right code waits");
    assert!(
        json_of(refused).await["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Too many wrong codes")
    );
    let off = app
        .api_post(
            "/api/ui/me/two-factor/off",
            json!({ "code": right }),
            Some(&tl),
        )
        .await;
    assert_eq!(off.status(), 429, "the account page shares the budget");
}

#[tokio::test]
async fn codes_sent_at_once_still_meet_the_budget() {
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    turn_on(&app, &tl).await;
    let into = now().rem_euclid(900);
    if into > 885 {
        tokio::time::sleep(std::time::Duration::from_secs((901 - into) as u64)).await;
    }
    let mut set = tokio::task::JoinSet::new();
    for i in 0..20 {
        let req = app
            .http
            .post(app.url("/api/ui/me/two-factor/off"))
            .header("origin", common::BASE_URL)
            .header("cookie", tl.clone())
            .json(&json!({ "code": format!("{i:06}") }));
        set.spawn(async move { req.send().await.unwrap().status().as_u16() });
    }
    let got = set.join_all().await;
    let wrong = got.iter().filter(|s| **s == 422).count();
    let refused = got.iter().filter(|s| **s == 429).count();
    assert_eq!((wrong, refused), (10, 10), "{got:?}");
}

/// A key shown at setup lasts 10 minutes (Q32): after that it cannot be confirmed, and the
/// retention job removes it.
#[tokio::test]
async fn a_waiting_key_expires_and_retention_clears_it() {
    let app = TestApp::spawn("test").await;
    let tl = app.tl("rina@x.co").await;
    let started = json_of(
        app.api_post("/api/ui/me/two-factor", json!({}), Some(&tl))
            .await,
    )
    .await;
    let secret = started["data"]["secret"].as_str().unwrap().to_string();
    let now = now();
    app.state
        .db
        .write(async |c| {
            c.execute(
                "UPDATE users SET totp_pending_expires_at = ?1 WHERE email = 'rina@x.co'",
                [now],
            )
            .await?;
            Ok(())
        })
        .await
        .unwrap();
    let code = code_at(&secret, now);
    let res = app
        .api_post(
            "/api/ui/me/two-factor/confirm",
            json!({ "code": code }),
            Some(&tl),
        )
        .await;
    assert_eq!(res.status(), 422);
    assert_eq!(
        json_of(res).await["error"]["message"],
        "This key expired or was replaced. Select Turn on to get a new one."
    );

    app.state
        .db
        .write(async |c| monita::accounts::purge_expired(c, now, 100).await)
        .await
        .unwrap();
    let left = monita::platform::db::query_i64(
        &app.state.db.read().await.unwrap(),
        "SELECT count(*) FROM users WHERE totp_pending IS NOT NULL",
        (),
    )
    .await
    .unwrap();
    assert_eq!(left, Some(0));
}
