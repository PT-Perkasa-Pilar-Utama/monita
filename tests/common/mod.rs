//! Starts a real Monita on a random port with a temp database and an in-memory mailer.
#![allow(dead_code)]

use std::net::SocketAddr;

use monita::{app::AppState, notify::Mailer, platform::config::Config};
use reqwest::{Response, header};

pub const RESET_TOKEN: &str = "test-reset-token";
pub const BASE_URL: &str = "https://monita.test";
pub const PASSWORD: &str = "correct-horse-battery";

pub struct TestApp {
    pub base: String,
    pub http: reqwest::Client,
    pub state: AppState,
    _dir: tempfile::TempDir,
}

impl TestApp {
    pub async fn spawn(env: &str) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("monita.db").to_str().unwrap().to_string();
        let env = env.to_string();
        let config = Config::from_lookup(|k| match k {
            "MONITA_ENV" => Some(env.clone()),
            "MONITA_BIND" => Some("127.0.0.1:0".into()),
            "MONITA_BASE_URL" => Some(BASE_URL.into()),
            "MONITA_DB_PATH" => Some(db.clone()),
            "MONITA_RESET_TOKEN" => Some(RESET_TOKEN.into()),
            _ => None,
        })
        .unwrap();
        let (router, state) = monita::app::build_with(config, Mailer::memory())
            .await
            .unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(
                listener,
                router.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await
            .unwrap();
        });
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();
        Self {
            base: format!("http://{addr}"),
            http,
            state,
            _dir: dir,
        }
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.base, path)
    }

    pub async fn get(&self, path: &str, cookie: Option<&str>) -> Response {
        let mut req = self.http.get(self.url(path));
        if let Some(c) = cookie {
            req = req.header(header::COOKIE, c);
        }
        req.send().await.unwrap()
    }

    /// A browser form post: carries Monita's origin, as a same-site browser would.
    pub async fn post_form(
        &self,
        path: &str,
        form: &[(&str, &str)],
        cookie: Option<&str>,
    ) -> Response {
        let mut req = self
            .http
            .post(self.url(path))
            .header(header::ORIGIN, BASE_URL)
            .form(form);
        if let Some(c) = cookie {
            req = req.header(header::COOKIE, c);
        }
        req.send().await.unwrap()
    }

    pub async fn super_admin(&self, email: &str) -> String {
        monita::accounts::create_super_admin(&self.state.db, email, PASSWORD)
            .await
            .unwrap();
        self.login(email, PASSWORD).await
    }

    /// Invites a TL as a super admin and accepts the invite. Returns the TL's session cookie.
    pub async fn tl(&self, email: &str) -> String {
        let link = monita::accounts::invite_tl(&self.state.db, BASE_URL, email)
            .await
            .unwrap();
        self.accept(link.strip_prefix(BASE_URL).unwrap(), PASSWORD)
            .await
    }

    /// Sets a password through an invite path. Returns the new session cookie.
    pub async fn accept(&self, invite_path: &str, password: &str) -> String {
        let res = self
            .post_form(
                invite_path,
                &[("password", password), ("password_confirm", password)],
                None,
            )
            .await;
        assert_eq!(res.status(), 303);
        cookie_of(&res)
    }

    pub async fn login(&self, email: &str, password: &str) -> String {
        let res = self
            .post_form("/login", &[("email", email), ("password", password)], None)
            .await;
        assert_eq!(res.status(), 303, "login failed for {email}");
        cookie_of(&res)
    }
}

/// `monita_session=...` from a response's Set-Cookie.
pub fn cookie_of(res: &Response) -> String {
    res.headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .find(|v| v.starts_with("monita_session="))
        .and_then(|v| v.split(';').next())
        .expect("no session cookie")
        .to_string()
}

pub fn location(res: &Response) -> String {
    res.headers()[header::LOCATION]
        .to_str()
        .unwrap()
        .to_string()
}

/// The invite link a page shows, by the `/invite/` path it contains.
pub fn invite_path(html: &str) -> String {
    let start = html.find("/invite/").expect("no invite link on the page");
    html[start..]
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '/' || *c == '-' || *c == '_')
        .collect()
}
