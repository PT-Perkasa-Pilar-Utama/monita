//! Project API keys on the wire: the `ApiProject` extractor for the ref-ID route, and the key's
//! principal for `/api/v1/projects/{slug}/` (v1.1 Q44). Both go through `authenticate`.

use axum::{
    extract::FromRequestParts,
    http::{StatusCode, header, request::Parts},
    response::Response,
};

use super::{KeyScope, Project, allowlist, store};
use crate::{
    accounts::{KeyGrant, User},
    app::AppState,
    platform::{api_error, now, token},
};

/// Where a project API key manages its project. A session cookie counts for nothing below it.
pub const KEY_API: &str = "/api/v1/projects/";

/// The project behind `Authorization: Bearer mk_...`, and what the key may do. Contract:
/// api-specs 03 section 3.2.
pub struct ApiProject(pub Project, pub super::KeyScope);

impl FromRequestParts<AppState> for ApiProject {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let key = authenticate(parts, state).await?;
        Ok(ApiProject(key.project, scope_of(&key.scope)))
    }
}

/// A scope the code does not know reads as the narrowest.
fn scope_of(raw: &str) -> KeyScope {
    KeyScope::parse(raw).unwrap_or(KeyScope::RefIds)
}

/// The principal of a key under `KEY_API`. The path's slug must be the key's own project, or
/// the answer is 404 before any handler runs, so a key never learns another project exists.
pub async fn key_user(parts: &Parts, state: &AppState) -> Result<User, Response> {
    let key = authenticate(parts, state).await?;
    let write = match scope_of(&key.scope) {
        KeyScope::RefIds => {
            return Err(api_error(
                StatusCode::FORBIDDEN,
                "forbidden",
                "This key may only issue ref IDs.",
            ));
        }
        KeyScope::Read => false,
        KeyScope::Write => true,
    };
    let slug = crate::platform::request_path(parts)[KEY_API.len()..]
        .split('/')
        .next();
    if slug != Some(key.project.slug.as_str()) {
        return Err(api_error(
            StatusCode::NOT_FOUND,
            "not_found",
            "This does not exist, or the key cannot open it.",
        ));
    }
    Ok(User::for_key(KeyGrant {
        project_id: key.project.id,
        name: key.name,
        write,
    }))
}

/// The live key in `Authorization: Bearer mk_...`, after the IP allowlist and the rate limits.
/// Contract: api-specs 03 section 3.2.
async fn authenticate(parts: &Parts, state: &AppState) -> Result<store::LiveKey, Response> {
    let unauthorized = || {
        api_error(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "Missing or invalid API key.",
        )
    };
    let raw = parts
        .headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        // RFC 7235: the scheme name is case-insensitive.
        .and_then(|v| v.split_once(' '))
        .filter(|(scheme, _)| scheme.eq_ignore_ascii_case("bearer"))
        .map(|(_, key)| key)
        .ok_or_else(unauthorized)?;
    let internal = |e: anyhow::Error| {
        tracing::error!(error = %e, "api key lookup failed");
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal",
            "Unexpected failure.",
        )
    };
    let ip =
        crate::platform::caller_ip(&parts.headers, &parts.extensions, state.config.trust_proxy);
    let conn = state.db.read().await.map_err(internal)?;
    // A wrong key has no key bucket yet, so its misses count per IP, as unknown ping tokens do.
    let miss_key = format!("keymiss:{}", crate::platform::rate_limit::ip_key(ip));
    let ip_limit = crate::platform::settings::get_or(&conn, "rl_ip_per_min", 60).await;
    let ip_limit = u32::try_from(ip_limit.max(1)).unwrap_or(u32::MAX);
    if let crate::platform::rate_limit::Verdict::Deny(retry) =
        state.limiter.over(&miss_key, ip_limit, 60, now())
    {
        return Err(crate::platform::rate_limit::too_many(
            crate::platform::request_path(parts),
            retry,
        ));
    }
    let Some(key) = store::find_live_key(&conn, &token::hash(raw.trim()))
        .await
        .map_err(internal)?
    else {
        state.limiter.hit(&miss_key, ip_limit, 60, now());
        return Err(unauthorized());
    };
    if let Some(list) = &key.project.ip_allowlist
        && !ip.is_some_and(|ip| allowlist::contains(list, ip))
    {
        return Err(api_error(
            StatusCode::FORBIDDEN,
            "forbidden",
            "This IP may not use this key.",
        ));
    }
    // SEC-12: per-key limit, read from settings on each call.
    let limit = crate::platform::settings::get_or(&conn, "rl_key_per_min", 120).await;
    drop(conn);
    if let crate::platform::rate_limit::Verdict::Deny(retry) = state.limiter.hit(
        &format!("key:{}", key.id),
        u32::try_from(limit.max(1)).unwrap_or(u32::MAX),
        60,
        now(),
    ) {
        return Err(crate::platform::rate_limit::too_many(
            crate::platform::request_path(parts),
            retry,
        ));
    }
    let t = now();
    if key.last_used_at.is_none_or(|at| t - at >= 60)
        && let Err(e) = state
            .db
            .write(async |c| store::touch_key(c, key.id, t).await)
            .await
    {
        tracing::warn!(key_id = key.id, error = %e, "could not record the key's last use");
    }
    Ok(key)
}
