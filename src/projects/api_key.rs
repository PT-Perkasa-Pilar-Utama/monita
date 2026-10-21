//! The API-key extractor: the project behind `Authorization: Bearer mk_...`.

use axum::{
    extract::FromRequestParts,
    http::{StatusCode, header, request::Parts},
    response::Response,
};

use super::{Project, allowlist, store};
use crate::{
    app::AppState,
    platform::{api_error, now, token},
};

/// The project behind `Authorization: Bearer mk_...`. Contract: api-specs 03 section 3.2.
pub struct ApiProject(pub Project);

impl FromRequestParts<AppState> for ApiProject {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
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
                parts.uri.path(),
                retry,
            ));
        }
        let Some((key_id, project, last_used)) =
            store::find_live_key(&conn, &token::hash(raw.trim()))
                .await
                .map_err(internal)?
        else {
            state.limiter.hit(&miss_key, ip_limit, 60, now());
            return Err(unauthorized());
        };
        if let Some(list) = &project.ip_allowlist
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
            &format!("key:{key_id}"),
            u32::try_from(limit.max(1)).unwrap_or(u32::MAX),
            60,
            now(),
        ) {
            return Err(crate::platform::rate_limit::too_many(
                parts.uri.path(),
                retry,
            ));
        }
        let t = now();
        if last_used.is_none_or(|at| t - at >= 60)
            && let Err(e) = state
                .db
                .write(async |c| store::touch_key(c, key_id, t).await)
                .await
        {
            tracing::warn!(key_id, error = %e, "could not record the key's last use");
        }
        Ok(ApiProject(project))
    }
}
