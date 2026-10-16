use askama::Template;
use axum::{Router, response::Response, routing::get};

use crate::{app::AppState, platform::render::render};

pub fn router() -> Router<AppState> {
    Router::new().route("/login", get(login_page))
}

#[derive(Template)]
#[template(path = "accounts/login.html")]
struct LoginPage<'a> {
    email: &'a str,
    error: Option<&'a str>,
}

async fn login_page() -> Response {
    render(&LoginPage {
        email: "",
        error: None,
    })
}
