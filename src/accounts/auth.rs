//! Sessions on the wire: the cookie and the `CurrentUser` and `AdminUser` extractors.

use axum::{
    extract::FromRequestParts,
    http::{HeaderMap, header, request::Parts},
    response::{IntoResponse, Redirect, Response},
};

use super::{Role, SESSION_COOKIE, SESSION_TTL_S, User, store};
use crate::{
    app::AppState,
    platform::{now, page::PageError, token},
};

pub fn session_cookie(raw: &str, secure: bool) -> String {
    format!(
        "{SESSION_COOKIE}={raw}; Path=/; HttpOnly; SameSite=Lax; Max-Age={SESSION_TTL_S}{}",
        if secure { "; Secure" } else { "" }
    )
}

pub fn clear_cookie() -> String {
    format!("{SESSION_COOKIE}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0")
}

pub fn session_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .find_map(|kv| {
            kv.trim()
                .strip_prefix(&format!("{SESSION_COOKIE}="))
                .map(str::to_owned)
        })
        .filter(|t| !t.is_empty())
}

/// The signed-in user. Without a valid session a page request goes to `/login`, and an `/api/`
/// request gets 401 in the JSON envelope. A supervisor gets 403 on any method but GET and HEAD:
/// every signed-in route that changes something takes this extractor, so this one check keeps
/// the role read-only. The one exception is a user's own two-factor (`AccountUser`).
pub struct CurrentUser(pub User);

impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let user = signed_in(parts, state).await?;
        if user.role == Role::Supervisor && !parts.method.is_safe() {
            return Err(forbidden(parts.uri.path().starts_with("/api/")));
        }
        Ok(CurrentUser(user))
    }
}

/// The signed-in user acting on their own sign-in, such as two-factor. Unlike `CurrentUser` it
/// lets a supervisor write: securing one's own account changes nothing anyone else sees.
pub struct AccountUser(pub User);

impl FromRequestParts<AppState> for AccountUser {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        Ok(AccountUser(signed_in(parts, state).await?))
    }
}

/// The user of the session cookie. Without a valid session a page request goes to `/login`, and
/// an `/api/` request gets 401 in the JSON envelope.
async fn signed_in(parts: &Parts, state: &AppState) -> Result<User, Response> {
    let api = parts.uri.path().starts_with("/api/");
    let to_login = || {
        if api {
            crate::platform::ui::UiError::Unauthorized.into_response()
        } else {
            Redirect::to("/login").into_response()
        }
    };
    let Some(raw) = session_token(&parts.headers) else {
        return Err(to_login());
    };
    let conn = state
        .db
        .read()
        .await
        .map_err(|e| PageError::from(e).into_response())?;
    match store::find_session_user(&conn, &token::hash(&raw), now()).await {
        Ok(Some(u)) => match Role::parse(&u.role) {
            Some(role) => Ok(User {
                id: u.id,
                email: u.email,
                role,
            }),
            None => Err(to_login()),
        },
        Ok(None) => Err(to_login()),
        Err(e) => Err(PageError::from(e).into_response()),
    }
}

fn forbidden(api: bool) -> Response {
    if api {
        crate::platform::ui::UiError::Forbidden.into_response()
    } else {
        PageError::Forbidden.into_response()
    }
}

/// A signed-in super admin, or a supervisor reading. Others get 403.
pub struct AdminUser(pub User);

impl FromRequestParts<AppState> for AdminUser {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let CurrentUser(user) = CurrentUser::from_request_parts(parts, state).await?;
        if user.sees_all() {
            Ok(AdminUser(user))
        } else {
            Err(forbidden(parts.uri.path().starts_with("/api/")))
        }
    }
}
