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
    assert_eq!(me["data"]["base_url"], BASE_URL);
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
    assert_eq!(
        app.get("/api/ui/admin/users", Some(&tl)).await.status(),
        403
    );
}

#[tokio::test]
async fn super_admin_invites_a_tl_and_the_email_goes_out() {
    // AC-02.01
    let app = TestApp::spawn("test").await;
    let admin = app.super_admin("admin@x.co").await;
    let res = app
        .api_post(
            "/api/ui/admin/users",
            json!({ "email": "rina@x.co" }),
            Some(&admin),
        )
        .await;
    assert_eq!(res.status(), 201);
    let path = invite_path(&res.text().await.unwrap());
    let list = json_of(app.get("/api/ui/admin/users", Some(&admin)).await).await;
    let rina = list["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["email"] == "rina@x.co")
        .unwrap();
    assert_eq!(rina["status"], "invited");
    assert!(
        !list.to_string().contains("password_hash"),
        "no hash leaves the server"
    );
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
        .api_post(
            &format!("/api/ui/admin/users/{rina}/invite"),
            json!({}),
            Some(&admin),
        )
        .await;
    let new_path = invite_path(&res.text().await.unwrap());

    assert_eq!(
        app.get("/api/ui/me", Some(&tl_session)).await.status(),
        401,
        "old session must end"
    );
    assert_eq!(app.get(&api_invite(&old_path), None).await.status(), 410);
    let before = app
        .api_post(
            "/api/ui/login",
            json!({ "email": "rina@x.co", "password": PASSWORD }),
            None,
        )
        .await;
    assert_eq!(
        before.status(),
        422,
        "the old password stops at once, not when the link is used"
    );
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
    app.api_post(
        &format!("/api/ui/admin/users/{id}/disable"),
        json!({}),
        Some(&admin),
    )
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
    app.api_post(
        &format!("/api/ui/admin/users/{id}/disable"),
        json!({}),
        Some(&admin),
    )
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
async fn an_admin_acts_on_others_but_never_on_their_own_account() {
    let app = TestApp::spawn("test").await;
    let admin = app.super_admin("admin@x.co").await;
    app.tl("rina@x.co").await;
    let admin_id = user_id(&app, "admin@x.co").await;
    let tl_id = user_id(&app, "rina@x.co").await;
    for action in ["invite", "disable"] {
        let res = app
            .api_post(
                &format!("/api/ui/admin/users/{admin_id}/{action}"),
                json!({}),
                Some(&admin),
            )
            .await;
        assert_eq!(res.status(), 422, "{action} on yourself");
    }
    let res = app
        .api_send(
            reqwest::Method::PUT,
            &format!("/api/ui/admin/users/{admin_id}/role"),
            json!({"role": "tl"}),
            Some(&admin),
        )
        .await;
    assert_eq!(res.status(), 422, "demoting yourself");
    app.api_post(
        &format!("/api/ui/admin/users/{tl_id}/disable"),
        json!({}),
        Some(&admin),
    )
    .await;
    let res = app
        .api_post(
            &format!("/api/ui/admin/users/{tl_id}/invite"),
            json!({}),
            Some(&admin),
        )
        .await;
    assert_eq!(res.status(), 404, "a disabled user gets no new invite");
}

#[tokio::test]
async fn a_promotion_and_a_demotion_hold_from_the_next_request() {
    let app = TestApp::spawn("test").await;
    let admin = app.super_admin("admin@x.co").await;
    let rina = app.tl("rina@x.co").await;
    let rina_id = user_id(&app, "rina@x.co").await;
    let users = "/api/ui/admin/users";
    assert_eq!(app.api_get(users, Some(&rina)).await.status(), 403);
    let role_path = format!("/api/ui/admin/users/{rina_id}/role");
    let set = |role: &'static str| {
        app.api_send(
            reqwest::Method::PUT,
            &role_path,
            json!({ "role": role }),
            Some(&admin),
        )
    };
    assert_eq!(set("super_admin").await.status(), 204);
    assert_eq!(
        app.api_get(users, Some(&rina)).await.status(),
        200,
        "the same session is admin now"
    );
    assert_eq!(set("tl").await.status(), 204);
    assert_eq!(app.api_get(users, Some(&rina)).await.status(), 403);
    assert_eq!(set("owner").await.status(), 422, "an unknown role");
}

#[tokio::test]
async fn a_disabled_super_admin_signs_in_again_once_enabled() {
    let app = TestApp::spawn("test").await;
    let admin = app.super_admin("admin@x.co").await;
    let res = app
        .api_post(
            "/api/ui/admin/users",
            json!({"email": "budi@x.co", "role": "super_admin"}),
            Some(&admin),
        )
        .await;
    assert_eq!(res.status(), 201);
    let link = json_of(res).await["data"]["link"]
        .as_str()
        .unwrap()
        .to_string();
    app.accept(link.strip_prefix(BASE_URL).unwrap(), PASSWORD)
        .await;
    let budi_id = user_id(&app, "budi@x.co").await;
    let disable = format!("/api/ui/admin/users/{budi_id}/disable");
    assert_eq!(
        app.api_post(&disable, json!({}), Some(&admin))
            .await
            .status(),
        204,
        "another super admin"
    );
    let refused = app
        .api_post(
            "/api/ui/login",
            json!({"email": "budi@x.co", "password": PASSWORD}),
            None,
        )
        .await;
    assert_ne!(refused.status(), 204, "disabled");
    let enable = format!("/api/ui/admin/users/{budi_id}/enable");
    assert_eq!(
        app.api_post(&enable, json!({}), Some(&admin))
            .await
            .status(),
        204
    );
    let back = app.login("budi@x.co", PASSWORD).await;
    assert_eq!(
        app.api_get("/api/ui/admin/users", Some(&back))
            .await
            .status(),
        200,
        "still a super admin"
    );
    assert_eq!(
        app.api_post(&enable, json!({}), Some(&admin))
            .await
            .status(),
        404,
        "already enabled"
    );
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

/// No account to sign in to (unknown, disabled with the right password, or invited and never
/// activated) gets the same refusal as a wrong password.
#[tokio::test]
async fn a_sign_in_without_a_usable_account_is_refused_like_a_wrong_password() {
    let app = TestApp::spawn("test").await;
    let admin = app.super_admin("admin@x.co").await;
    app.tl("rina@x.co").await;
    monita::accounts::invite_tl(&app.state.db, common::BASE_URL, "new@x.co")
        .await
        .unwrap();
    let users = common::json_of(app.api_get("/api/ui/admin/users", Some(&admin)).await).await;
    let rina = users["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["email"] == "rina@x.co")
        .unwrap()["id"]
        .as_i64()
        .unwrap();
    let off = app
        .api_post(
            &format!("/api/ui/admin/users/{rina}/disable"),
            json!({}),
            Some(&admin),
        )
        .await;
    assert_eq!(off.status(), 204);
    let login = async |email: &str, password: &str| {
        let res = app
            .api_post(
                "/api/ui/login",
                json!({ "email": email, "password": password }),
                None,
            )
            .await;
        (
            res.status(),
            common::json_of(res).await["error"]["message"].clone(),
        )
    };
    let wrong = login("admin@x.co", "not-the-password").await;
    assert_eq!(wrong.0, 422);
    for email in ["nobody@x.co", "rina@x.co", "new@x.co"] {
        assert_eq!(login(email, PASSWORD).await, wrong, "{email}");
    }
}

/// Two super admins acting on each other at once: the second write finds its actor already
/// disabled or demoted, so one active super admin always remains.
#[tokio::test]
async fn an_admin_disabled_or_demoted_meanwhile_cannot_act_on_the_other() {
    let app = TestApp::spawn("test").await;
    app.super_admin("a@x.co").await;
    app.super_admin("b@x.co").await;
    let id = async |email: &str| {
        monita::platform::db::query_i64(
            &app.state.db.read().await.unwrap(),
            "SELECT id FROM users WHERE email = ?1",
            [email],
        )
        .await
        .unwrap()
        .unwrap()
    };
    let (a, b) = (id("a@x.co").await, id("b@x.co").await);
    // A's change lands first; B's request had already passed its role check.
    accounts::disable(&app.state.db, a, b).await.unwrap();
    assert!(matches!(
        accounts::disable(&app.state.db, b, a).await,
        Err(accounts::AdminError::NotSuperAdmin)
    ));
    accounts::enable(&app.state.db, b).await.unwrap();
    accounts::set_role(&app.state.db, a, b, accounts::Role::Tl)
        .await
        .unwrap();
    assert!(matches!(
        accounts::set_role(&app.state.db, b, a, accounts::Role::Tl).await,
        Err(accounts::AdminError::NotSuperAdmin)
    ));
    let admins = monita::platform::db::query_i64(
        &app.state.db.read().await.unwrap(),
        "SELECT count(*) FROM users WHERE role = 'super_admin' AND disabled_at IS NULL",
        (),
    )
    .await
    .unwrap();
    assert_eq!(admins, Some(1));
}
