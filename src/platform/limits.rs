//! `/admin/settings`: the runtime limits a super admin can change (AC-11.03, AC-31.03).

use askama::Template;
use axum::{
    Form, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use std::collections::HashMap;

use super::{
    page::{PageError, Shell},
    render::render,
    settings,
};
use crate::{accounts::AdminUser, app::AppState};

pub fn router() -> Router<AppState> {
    Router::new().route("/admin/settings", get(page).post(save))
}

#[derive(Template)]
#[template(path = "platform/limits.html")]
struct LimitsPage {
    shell: Shell,
    rows: Vec<(&'static str, &'static str, String)>,
    error: String,
}

async fn view(
    state: &AppState,
    email: String,
    error: String,
    posted: Option<&HashMap<String, String>>,
) -> Result<LimitsPage, PageError> {
    let current = settings::all(&state.db.read().await?).await?;
    let rows = settings::KEYS
        .iter()
        .map(|(key, label)| {
            let value = posted
                .and_then(|p| p.get(*key).cloned())
                .or_else(|| {
                    current
                        .iter()
                        .find(|(k, _)| k == key)
                        .map(|(_, v)| v.to_string())
                })
                .unwrap_or_default();
            (*key, *label, value)
        })
        .collect();
    Ok(LimitsPage {
        shell: Shell {
            email,
            is_admin: true,
            active: "limits",
            project: None,
        },
        rows,
        error,
    })
}

async fn page(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
) -> Result<Response, PageError> {
    Ok(render(
        &view(&state, admin.email, String::new(), None).await?,
    ))
}

async fn save(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
    Form(form): Form<HashMap<String, String>>,
) -> Result<Response, PageError> {
    let mut parsed = Vec::new();
    for (key, label) in settings::KEYS {
        match form
            .get(*key)
            .and_then(|v| v.trim().parse::<i64>().ok())
            .filter(|v| *v > 0)
        {
            Some(v) => parsed.push((*key, v)),
            None => {
                let page = view(
                    &state,
                    admin.email,
                    format!("{label}: enter a whole number above 0."),
                    Some(&form),
                )
                .await?;
                return Ok((StatusCode::UNPROCESSABLE_ENTITY, render(&page)).into_response());
            }
        }
    }
    state
        .db
        .write(async |c| {
            for (key, value) in &parsed {
                settings::set(c, key, *value).await?;
            }
            Ok(())
        })
        .await?;
    Ok(Redirect::to("/admin/settings").into_response())
}
