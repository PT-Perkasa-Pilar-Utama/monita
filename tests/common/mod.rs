//! Starts a real Monita on a random port with a temp database and an in-memory mailer.
#![allow(dead_code)]

use std::net::SocketAddr;

use monita::{app::AppState, notify::Mailer, platform::config::Config};
use reqwest::{Response, header};
use serde_json::{Value, json};

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
        Self::spawn_with(env, &[]).await
    }

    /// `extra` sets more `MONITA_*` variables, for example the backup folder.
    pub async fn spawn_with(env: &str, extra: &[(&str, &str)]) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("monita.db").to_str().unwrap().to_string();
        let env = env.to_string();
        let extra: Vec<(String, String)> = extra
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        let config = Config::from_lookup(|k| match k {
            "MONITA_ENV" => Some(env.clone()),
            "MONITA_BIND" => Some("127.0.0.1:0".into()),
            "MONITA_BASE_URL" => Some(BASE_URL.into()),
            "MONITA_DB_PATH" => Some(db.clone()),
            "MONITA_RESET_TOKEN" => Some(RESET_TOKEN.into()),
            other => extra
                .iter()
                .find(|(key, _)| key == other)
                .map(|(_, v)| v.clone()),
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

    /// Creates a project through the frontend's API. Monita makes the slug from the name plus a
    /// random suffix; the test then sets `slug` so its URLs stay readable. Returns the status:
    /// 201, or 422 for a refused form.
    pub async fn project(&self, cookie: &str, name: &str, slug: &str) -> reqwest::StatusCode {
        let res = self
            .api_post(
                "/api/ui/projects",
                json!({ "name": name, "homepage_url": "https://app.example.com" }),
                Some(cookie),
            )
            .await;
        let status = res.status();
        if status == 201 {
            let made = json_of(res).await["data"]["slug"]
                .as_str()
                .unwrap()
                .to_string();
            let slug = slug.to_string();
            self.state
                .db
                .write(async |c| {
                    c.execute(
                        "UPDATE projects SET slug = ?1 WHERE slug = ?2",
                        (slug, made),
                    )
                    .await?;
                    Ok(())
                })
                .await
                .unwrap();
        }
        status
    }

    /// A frontend read: `GET /api/ui/...` with the session cookie.
    pub async fn api_get(&self, path: &str, cookie: Option<&str>) -> Response {
        self.get(path, cookie).await
    }

    /// A frontend write: JSON with Monita's origin, as the browser's `fetch` sends it.
    pub async fn api_send(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Value,
        cookie: Option<&str>,
    ) -> Response {
        let mut req = self
            .http
            .request(method, self.url(path))
            .header(header::ORIGIN, BASE_URL)
            .json(&body);
        if let Some(c) = cookie {
            req = req.header(header::COOKIE, c);
        }
        req.send().await.unwrap()
    }

    pub async fn api_post(&self, path: &str, body: Value, cookie: Option<&str>) -> Response {
        self.api_send(reqwest::Method::POST, path, body, cookie)
            .await
    }

    /// Adds an alert card to a project as the page does; returns its id.
    pub async fn alert_card(&self, cookie: &str, slug: &str, card: Value) -> i64 {
        let res = self
            .api_post(
                &format!("/api/ui/projects/{slug}/alert-cards"),
                card,
                Some(cookie),
            )
            .await;
        assert_eq!(
            res.status(),
            201,
            "{}",
            res.text().await.unwrap_or_default()
        );
        json_of(res).await["data"]["id"].as_i64().unwrap()
    }

    /// Turns the owner's two email cards off, as a team that replaced the owner would.
    pub async fn owner_cards_off(&self, cookie: &str, slug: &str) {
        for card in self.alert_cards(cookie, slug).await.as_array().unwrap() {
            if card["fixed"] == true {
                let res = self
                    .api_send(
                        reqwest::Method::PUT,
                        &format!("/api/ui/projects/{slug}/alert-cards/{}/enabled", card["id"]),
                        json!({ "enabled": false }),
                        Some(cookie),
                    )
                    .await;
                assert_eq!(res.status(), 204);
            }
        }
    }

    /// Sends `level` alerts to `email` alone: the owner's cards go off and an Email card takes it.
    pub async fn email_only(&self, cookie: &str, slug: &str, level: &str, email: &str) -> i64 {
        self.owner_cards_off(cookie, slug).await;
        self.alert_card(
            cookie,
            slug,
            json!({ "kind": "email", "name": "Email", "emails": email, level: true }),
        )
        .await
    }

    /// A project's cards as the page reads them.
    pub async fn alert_cards(&self, cookie: &str, slug: &str) -> Value {
        json_of(
            self.api_get(
                &format!("/api/ui/projects/{slug}/alert-cards"),
                Some(cookie),
            )
            .await,
        )
        .await["data"]
            .clone()
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

    /// Sets a password through an invite path (`/invite/<token>`). Returns the session cookie.
    pub async fn accept(&self, invite_path: &str, password: &str) -> String {
        let res = self
            .api_post(
                &api_invite(invite_path),
                json!({ "password": password, "password_confirm": password }),
                None,
            )
            .await;
        assert_eq!(res.status(), 204);
        cookie_of(&res)
    }

    pub async fn login(&self, email: &str, password: &str) -> String {
        let res = self
            .api_post(
                "/api/ui/login",
                json!({ "email": email, "password": password }),
                None,
            )
            .await;
        assert_eq!(res.status(), 204, "login failed for {email}");
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

/// The invite path (`/invite/<token>`) inside a response body, such as the admin invite JSON.
pub fn invite_path(body: &str) -> String {
    let start = body.find("/invite/").expect("no invite link in the body");
    body[start..]
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '/' || *c == '-' || *c == '_')
        .collect()
}

/// The JSON body of a response.
pub async fn json_of(res: Response) -> Value {
    res.json().await.expect("a JSON body")
}

/// The invite API path for a page path `/invite/<token>`.
pub fn api_invite(invite_path: &str) -> String {
    format!(
        "/api/ui/invites/{}",
        invite_path.trim_start_matches("/invite/")
    )
}
