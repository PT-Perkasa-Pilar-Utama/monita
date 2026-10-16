//! Starts a real Monita on a random port with a temp database.

use std::net::SocketAddr;

use monita::platform::config::Config;

pub struct TestApp {
    pub base: String,
    pub http: reqwest::Client,
    _dir: tempfile::TempDir,
}

pub const RESET_TOKEN: &str = "test-reset-token";

impl TestApp {
    pub async fn spawn(env: &str) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("monita.db");
        let db = db_path.to_str().unwrap().to_string();
        let env = env.to_string();
        let config = Config::from_lookup(|k| match k {
            "MONITA_ENV" => Some(env.clone()),
            "MONITA_BIND" => Some("127.0.0.1:0".into()),
            "MONITA_BASE_URL" => Some("https://monita.test".into()),
            "MONITA_DB_PATH" => Some(db.clone()),
            "MONITA_RESET_TOKEN" => Some(RESET_TOKEN.into()),
            _ => None,
        })
        .unwrap();
        let (router, _state) = monita::app::build(config).await.unwrap();
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
        Self {
            base: format!("http://{addr}"),
            http: reqwest::Client::new(),
            _dir: dir,
        }
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.base, path)
    }
}
