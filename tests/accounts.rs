//! US-01 to US-04. AC: docs/business/acceptance-criteria-breakdown/acceptance-criteria-sprint-1.md.

mod common;

use common::{BASE_URL, PASSWORD, TestApp, api_invite, cookie_of, invite_path, json_of};
use monita::accounts::{self, AccountError};
use serde_json::json;

#[tokio::test]
async fn super_admin_from_the_cli_can_sign_in() {
    // AC-01.01
    let app = TestApp::spawn("test").await;
    let cookie = app.super_admin("Admin@Company.co.id").await;
    let me = app.get("/api/ui/me", Some(&cookie)).await;
    assert_eq!(me.status(), 200);
    let me = json_of(me).await;
    assert_eq!(me["data"]["email"], "admin@company.co.id");
    assert_eq!(me["data"]["role"], "super_admin");
}

#[tokio::test]
async fn cli_rejects_an_email_that_exists() {
    // AC-01.02
    let app = TestApp::spawn("test").await;
    accounts::create_super_admin(&app.state.db, "a@x.co", PASSWORD)
        .await
        .unwrap();
    let again = accounts::create_super_admin(&app.state.db, "A@X.co", PASSWORD).await;
    assert!(matches!(again, Err(AccountError::EmailTaken)));
}

#[tokio::test]
async fn wrong_password_is_refused_without_saying_which_part() {
    let app = TestApp::spawn("test").await;
    accounts::create_super_admin(&app.state.db, "a@x.co", PASSWORD)
        .await
        .unwrap();
    let res = app
        .api_post(
            "/api/ui/login",
            json!({ "email": "a@x.co", "password": "wrong-password-123" }),
            None,
        )
        .await;
    assert_eq!(res.status(), 422);
    assert!(res.headers().get("set-cookie").is_none());
    assert_eq!(
        json_of(res).await["error"]["message"],
        "Wrong email or password."
    );
}

#[tokio::test]
async fn pages_need_a_session_and_admin_pages_need_a_super_admin() {
    let app = TestApp::spawn("test").await;
    let anon = app.get("/api/ui/me", None).await;
    assert_eq!(anon.status(), 401);
    assert_eq!(json_of(anon).await["error"]["code"], "unauthorized");
    let tl = app.tl("tl@x.co").await;
    assert_eq!(app.get("/admin/users", Some(&tl)).await.status(), 403);
}

#[tokio::test]
async fn super_admin_invites_a_tl_and_the_email_goes_out() {
    // AC-02.01
    let app = TestApp::spawn("test").await;
    let admin = app.super_admin("admin@x.co").await;
    let res = app
        .post_form("/admin/users", &[("email", "rina@x.co")], Some(&admin))
        .await;
    assert_eq!(res.status(), 200);
    let path = invite_path(&res.text().await.unwrap());
    // The scheduler delivers too, so wait until the row is marked sent rather than count one pass:
    // the body is cleared in that same mark.
    monita::notify::deliver_due(&app.state).await.unwrap();
    let sent_rows = async || {
        monita::platform::db::query_i64(
            &app.state.db.read().await.unwrap(),
            "SELECT count(*) FROM outbox WHERE sent_at IS NOT NULL",
            (),
        )
        .await
        .unwrap()
    };
    for _ in 0..20 {
        if sent_rows().await == Some(1) {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    let emails = app.state.mailer.sent();
    assert_eq!(emails.len(), 1);
    assert_eq!(emails[0].to, "rina@x.co");
    assert!(emails[0].body.contains(&format!("{BASE_URL}{path}")));
    let kept = monita::platform::db::query_i64(
        &app.state.db.read().await.unwrap(),
        "SELECT count(*) FROM outbox WHERE body != ''",
        (),
    )
    .await
    .unwrap();
    assert_eq!(kept, Some(0), "the invite link does not stay in the outbox");
}

#[tokio::test]
async fn invite_link_sets_a_password_once_and_signs_in() {
    // AC-02.02, AC-03.01, AC-03.02, AC-03.03
    let app = TestApp::spawn("test").await;
    let link = accounts::invite_tl(&app.state.db, BASE_URL, "rina@x.co")
        .await
        .unwrap();
    let path = link.strip_prefix(BASE_URL).unwrap();
    assert_eq!(app.get(&api_invite(path), None).await.status(), 200);

    let res = app
        .api_post(
            &api_invite(path),
            json!({ "password": PASSWORD, "password_confirm": PASSWORD }),
            None,
        )
        .await;
    assert_eq!(res.status(), 204);
    let cookie = cookie_of(&res);
    assert_eq!(app.get("/api/ui/me", Some(&cookie)).await.status(), 200);

    assert_eq!(app.get(&api_invite(path), None).await.status(), 410);
    let reuse = app
        .api_post(
            &api_invite(path),
            json!({ "password": PASSWORD, "password_confirm": PASSWORD }),
            None,
        )
        .await;
    assert_eq!(reuse.status(), 410);
}

#[tokio::test]
async fn new_invite_replaces_the_password_and_kills_old_links_and_sessions() {
    // AC-04.01, AC-04.02
    let app = TestApp::spawn("test").await;
    let admin = app.super_admin("admin@x.co").await;
    let first = accounts::invite_tl(&app.state.db, BASE_URL, "rina@x.co")
        .await
        .unwrap();
    let old_path = first.strip_prefix(BASE_URL).unwrap().to_string();
    let tl_session = app.accept(&old_path, PASSWORD).await;

    let users = accounts::list_users(&app.state.db).await.unwrap();
    let rina = users.iter().find(|u| u.email == "rina@x.co").unwrap().id;
    let res = app
        .post_form(&format!("/admin/users/{rina}/invite"), &[], Some(&admin))
        .await;
    let new_path = invite_path(&res.text().await.unwrap());

    assert_eq!(
        app.get("/api/ui/me", Some(&tl_session)).await.status(),
        401,
        "old session must end"
    );
    assert_eq!(app.get(&api_invite(&old_path), None).await.status(), 410);
    app.accept(&new_path, "a-brand-new-password").await;
    let old_pw = app
        .api_post(
            "/api/ui/login",
            json!({ "email": "rina@x.co", "password": PASSWORD }),
            None,
        )
        .await;
    assert_eq!(old_pw.status(), 422);
    app.login("rina@x.co", "a-brand-new-password").await;
}

#[tokio::test]
async fn a_short_password_or_a_mismatch_is_refused() {
    let app = TestApp::spawn("test").await;
    let link = accounts::invite_tl(&app.state.db, BASE_URL, "rina@x.co")
        .await
        .unwrap();
    let path = link.strip_prefix(BASE_URL).unwrap();
    let short = app
        .api_post(
            &api_invite(path),
            json!({ "password": "short", "password_confirm": "short" }),
            None,
        )
        .await;
    assert_eq!(short.status(), 422);
    let mismatch = app
        .api_post(
            &api_invite(path),
            json!({ "password": PASSWORD, "password_confirm": "something-else-1" }),
            None,
        )
        .await;
    assert_eq!(mismatch.status(), 422);
    assert_eq!(
        app.get(&api_invite(path), None).await.status(),
        200,
        "a refused attempt leaves the link usable"
    );
}

#[tokio::test]
async fn a_form_post_from_another_origin_is_blocked() {
    // SEC-03
    let app = TestApp::spawn("test").await;
    accounts::create_super_admin(&app.state.db, "a@x.co", PASSWORD)
        .await
        .unwrap();
    let body = json!({ "email": "a@x.co", "password": PASSWORD });
    let evil = app
        .http
        .post(app.url("/api/ui/login"))
        .header("origin", "https://evil.example")
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(evil.status(), 403);
    let none = app
        .http
        .post(app.url("/api/ui/login"))
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(
        none.status(),
        403,
        "a write without an Origin is blocked too"
    );
    assert_eq!(json_of(none).await["error"]["code"], "forbidden");
}

#[tokio::test]
async fn disabled_tl_is_signed_out_and_cannot_sign_in() {
    let app = TestApp::spawn("test").await;
    let admin = app.super_admin("admin@x.co").await;
    let tl = app.tl("rina@x.co").await;
    let id = accounts::list_users(&app.state.db)
        .await
        .unwrap()
        .into_iter()
        .find(|u| u.email == "rina@x.co")
        .unwrap()
        .id;
    app.post_form(&format!("/admin/users/{id}/disable"), &[], Some(&admin))
        .await;
    assert_eq!(app.get("/api/ui/me", Some(&tl)).await.status(), 401);
    let res = app
        .api_post(
            "/api/ui/login",
            json!({ "email": "rina@x.co", "password": PASSWORD }),
            None,
        )
        .await;
    assert_eq!(res.status(), 422);
}

async fn user_id(app: &TestApp, email: &str) -> i64 {
    accounts::list_users(&app.state.db)
        .await
        .unwrap()
        .into_iter()
        .find(|u| u.email == email)
        .unwrap()
        .id
}

#[tokio::test]
async fn disabling_a_tl_kills_their_pending_invite() {
    let app = TestApp::spawn("test").await;
    let admin = app.super_admin("admin@x.co").await;
    let link = accounts::invite_tl(&app.state.db, BASE_URL, "rina@x.co")
        .await
        .unwrap();
    let path = link.strip_prefix(BASE_URL).unwrap();
    let id = user_id(&app, "rina@x.co").await;
    app.post_form(&format!("/admin/users/{id}/disable"), &[], Some(&admin))
        .await;
    assert_eq!(app.get(&api_invite(path), None).await.status(), 410);
    let res = app
        .api_post(
            &api_invite(path),
            json!({ "password": PASSWORD, "password_confirm": PASSWORD }),
            None,
        )
        .await;
    assert_eq!(res.status(), 410);
    let user = accounts::list_users(&app.state.db)
        .await
        .unwrap()
        .into_iter()
        .find(|u| u.id == id)
        .unwrap();
    assert!(user.disabled_at.is_some() && user.password_hash.is_none());
}

#[tokio::test]
async fn reinvite_and_disable_refuse_anyone_but_an_active_tl() {
    let app = TestApp::spawn("test").await;
    let admin = app.super_admin("admin@x.co").await;
    app.tl("rina@x.co").await;
    let admin_id = user_id(&app, "admin@x.co").await;
    let tl_id = user_id(&app, "rina@x.co").await;
    for action in ["invite", "disable"] {
        let res = app
            .post_form(
                &format!("/admin/users/{admin_id}/{action}"),
                &[],
                Some(&admin),
            )
            .await;
        assert_eq!(res.status(), 404, "{action} on a super admin");
    }
    app.post_form(&format!("/admin/users/{tl_id}/disable"), &[], Some(&admin))
        .await;
    let res = app
        .post_form(&format!("/admin/users/{tl_id}/invite"), &[], Some(&admin))
        .await;
    assert_eq!(res.status(), 404, "a disabled TL gets no new invite");
}

#[tokio::test]
async fn invite_posts_are_limited_per_ip_and_a_bad_token_is_refused_before_hashing() {
    let app = TestApp::spawn("test").await;
    app.state
        .db
        .write(async |c| monita::platform::settings::set(c, "rl_ip_per_min", 2).await)
        .await
        .unwrap();
    let form = json!({ "password": PASSWORD, "password_confirm": "a-different-one" });
    for _ in 0..2 {
        let res = app
            .api_post("/api/ui/invites/not-a-token", form.clone(), None)
            .await;
        assert_eq!(res.status(), 410, "a dead link is 410 even with a mismatch");
    }
    let res = app
        .api_post("/api/ui/invites/not-a-token", form, None)
        .await;
    assert_eq!(res.status(), 429);
}

#[tokio::test]
async fn the_cli_resets_a_super_admin_password_and_signs_them_out() {
    let app = TestApp::spawn("test").await;
    let old_session = app.super_admin("admin@x.co").await;
    app.tl("rina@x.co").await;
    let new_password = "a-brand-new-password";
    accounts::reset_super_admin_password(&app.state.db, "Admin@X.co", new_password)
        .await
        .unwrap();
    assert_eq!(
        app.get("/api/ui/me", Some(&old_session)).await.status(),
        401
    );
    for (password, status) in [(PASSWORD, 422), (new_password, 204)] {
        let res = app
            .api_post(
                "/api/ui/login",
                json!({ "email": "admin@x.co", "password": password }),
                None,
            )
            .await;
        assert_eq!(res.status(), status, "password {password}");
    }
    let tl = accounts::reset_super_admin_password(&app.state.db, "rina@x.co", new_password).await;
    assert!(
        matches!(tl, Err(AccountError::NoSuchAdmin)),
        "a TL resets through an invite"
    );
}

#[tokio::test]
async fn an_existing_address_signs_in_even_if_new_ones_would_be_refused() {
    let app = TestApp::spawn("test").await;
    accounts::create_super_admin(&app.state.db, "jorg@x.co", PASSWORD)
        .await
        .unwrap();
    app.state
        .db
        .write(async |c| {
            c.execute("UPDATE users SET email = 'jörg@x.co'", ())
                .await?;
            Ok(())
        })
        .await
        .unwrap();
    let res = app
        .api_post(
            "/api/ui/login",
            json!({ "email": " Jörg@X.co", "password": PASSWORD }),
            None,
        )
        .await;
    assert_eq!(res.status(), 204);
}

#[tokio::test]
async fn a_corrupt_stored_hash_refuses_the_sign_in_without_a_crash() {
    let app = TestApp::spawn("test").await;
    accounts::create_super_admin(&app.state.db, "a@x.co", PASSWORD)
        .await
        .unwrap();
    app.state
        .db
        .write(async |c| {
            c.execute("UPDATE users SET password_hash = 'not-a-phc-string'", ())
                .await?;
            Ok(())
        })
        .await
        .unwrap();
    let res = app
        .api_post(
            "/api/ui/login",
            json!({ "email": "a@x.co", "password": PASSWORD }),
            None,
        )
        .await;
    assert_eq!(res.status(), 422);
}
