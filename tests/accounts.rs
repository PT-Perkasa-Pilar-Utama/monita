//! US-01 to US-04. AC: docs/business/acceptance-criteria-breakdown/acceptance-criteria-sprint-1.md.

mod common;

use common::{BASE_URL, PASSWORD, TestApp, cookie_of, invite_path, location};
use monita::accounts::{self, AccountError};

#[tokio::test]
async fn super_admin_from_the_cli_can_sign_in() {
    // AC-01.01
    let app = TestApp::spawn("test").await;
    let cookie = app.super_admin("Admin@Company.co.id").await;
    let home = app.get("/", Some(&cookie)).await;
    assert_eq!(home.status(), 200);
    assert!(home.text().await.unwrap().contains("admin@company.co.id"));
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
        .post_form(
            "/login",
            &[("email", "a@x.co"), ("password", "wrong-password-123")],
            None,
        )
        .await;
    assert_eq!(res.status(), 422);
    assert!(res.headers().get("set-cookie").is_none());
    assert!(
        res.text()
            .await
            .unwrap()
            .contains("Wrong email or password.")
    );
}

#[tokio::test]
async fn pages_need_a_session_and_admin_pages_need_a_super_admin() {
    let app = TestApp::spawn("test").await;
    let anon = app.get("/", None).await;
    assert_eq!(anon.status(), 303);
    assert_eq!(location(&anon), "/login");
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
    let sent = monita::notify::deliver_due(&app.state).await.unwrap();
    assert_eq!(sent, 1);
    let emails = app.state.mailer.sent();
    assert_eq!(emails[0].to, "rina@x.co");
    assert!(emails[0].body.contains(&format!("{BASE_URL}{path}")));
}

#[tokio::test]
async fn invite_link_sets_a_password_once_and_signs_in() {
    // AC-02.02, AC-03.01, AC-03.02, AC-03.03
    let app = TestApp::spawn("test").await;
    let link = accounts::invite_tl(&app.state.db, BASE_URL, "rina@x.co")
        .await
        .unwrap();
    let path = link.strip_prefix(BASE_URL).unwrap();
    let form = app.get(path, None).await;
    assert_eq!(form.status(), 200);
    assert!(form.text().await.unwrap().contains("Set your password"));

    let res = app
        .post_form(
            path,
            &[("password", PASSWORD), ("password_confirm", PASSWORD)],
            None,
        )
        .await;
    assert_eq!(res.status(), 303);
    let cookie = cookie_of(&res);
    assert_eq!(app.get("/", Some(&cookie)).await.status(), 200);

    assert_eq!(app.get(path, None).await.status(), 410);
    let reuse = app
        .post_form(
            path,
            &[("password", PASSWORD), ("password_confirm", PASSWORD)],
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
        app.get("/", Some(&tl_session)).await.status(),
        303,
        "old session must end"
    );
    assert_eq!(app.get(&old_path, None).await.status(), 410);
    app.accept(&new_path, "a-brand-new-password").await;
    let old_pw = app
        .post_form(
            "/login",
            &[("email", "rina@x.co"), ("password", PASSWORD)],
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
        .post_form(
            path,
            &[("password", "short"), ("password_confirm", "short")],
            None,
        )
        .await;
    assert_eq!(short.status(), 422);
    let mismatch = app
        .post_form(
            path,
            &[
                ("password", PASSWORD),
                ("password_confirm", "something-else-1"),
            ],
            None,
        )
        .await;
    assert_eq!(mismatch.status(), 422);
    assert_eq!(
        app.get(path, None).await.status(),
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
    let res = app
        .http
        .post(app.url("/login"))
        .header("origin", "https://evil.example")
        .form(&[("email", "a@x.co"), ("password", PASSWORD)])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 403);
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
    assert_eq!(app.get("/", Some(&tl)).await.status(), 303);
    let res = app
        .post_form(
            "/login",
            &[("email", "rina@x.co"), ("password", PASSWORD)],
            None,
        )
        .await;
    assert_eq!(res.status(), 422);
}
