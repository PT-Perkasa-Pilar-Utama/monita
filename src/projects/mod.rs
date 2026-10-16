//! projects module: projects, ownership, API keys, IP allowlist.
//! Spec: docs/technical-specs/05-module-definitions.md section 5.2.

mod allowlist;
pub mod routes;
mod store;

use axum::{
    extract::FromRequestParts,
    http::{StatusCode, header, request::Parts},
    response::Response,
};

use crate::{
    accounts::User,
    app::AppState,
    platform::{api_error, db::Db, now, page::PageError, token},
};

pub use store::{ApiKeyRow, Project};

#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    #[error("Enter a name.")]
    InvalidName,
    #[error("Use 2 to 40 lowercase letters, digits or dashes.")]
    InvalidSlug,
    #[error("Slug already used.")]
    SlugTaken,
    #[error("Not a valid IP or CIDR: {0}")]
    InvalidCidr(String),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub fn valid_slug(slug: &str) -> bool {
    (2..=40).contains(&slug.len())
        && slug
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// US-06: creates the project with the caller as owner, plus its two alert routes.
pub async fn create(
    db: &Db,
    owner: &User,
    name: &str,
    slug: &str,
) -> Result<Project, ProjectError> {
    let name = name.trim();
    let slug = slug.trim().to_lowercase();
    if name.is_empty() || name.chars().count() > 80 {
        return Err(ProjectError::InvalidName);
    }
    if !valid_slug(&slug) {
        return Err(ProjectError::InvalidSlug);
    }
    let created = db
        .write(async |c| {
            if store::find_by_slug(c, &slug).await?.is_some() {
                return Ok(false);
            }
            let id = store::insert(c, owner.id, name, &slug).await?;
            crate::notify::create_routes(c, id).await?;
            Ok(true)
        })
        .await?;
    if !created {
        return Err(ProjectError::SlugTaken);
    }
    store::find_by_slug(&db.read().await?, &slug)
        .await?
        .ok_or_else(|| ProjectError::Other(anyhow::anyhow!("project {slug} vanished after insert")))
}

pub async fn list_for(db: &Db, user: &User) -> anyhow::Result<Vec<Project>> {
    let conn = db.read().await?;
    if user.is_admin() {
        store::list_all(&conn).await
    } else {
        store::list_owned(&conn, user.id).await
    }
}

/// Every project, for jobs that run across projects (the digest).
pub async fn all(db: &Db) -> anyhow::Result<Vec<Project>> {
    store::list_all(&db.read().await?).await
}

/// The page frame for a signed-in user: who they are, the active nav item, the project in view.
pub fn shell(
    user: &User,
    active: &'static str,
    project: Option<&Project>,
) -> crate::platform::page::Shell {
    let shell = crate::platform::page::Shell::plain(&user.email, user.is_admin(), active);
    match project {
        Some(p) => shell.in_project(&p.name, &p.slug),
        None => shell,
    }
}

/// Who `reassign` accepts, for the owner list: the acting super admin first, then active TLs.
pub async fn eligible_owners(
    db: &Db,
    acting: &User,
) -> anyhow::Result<Vec<crate::accounts::UserRow>> {
    let mut all = crate::accounts::list_users(db).await?;
    all.retain(|u| u.id == acting.id || (u.role == "tl" && u.disabled_at.is_none()));
    all.sort_by_key(|u| u.id != acting.id);
    Ok(all)
}

pub fn can_manage(user: &User, project: &Project) -> bool {
    user.is_admin() || project.owner_id == user.id
}

/// The project a user may manage, or 404 so other projects' names do not leak.
pub async fn find_for(db: &Db, user: &User, slug: &str) -> Result<Project, PageError> {
    let project = store::find_by_slug(&db.read().await?, slug).await?;
    match project {
        Some(p) if can_manage(user, &p) => Ok(p),
        _ => Err(PageError::NotFound),
    }
}

pub async fn find_public(db: &Db, slug: &str) -> anyhow::Result<Option<Project>> {
    store::find_by_slug(&db.read().await?, slug).await
}

/// Name and slug for messages, inside a caller's transaction.
pub async fn name_slug(
    c: &turso::Connection,
    project_id: i64,
) -> anyhow::Result<Option<(String, String)>> {
    store::name_slug(c, project_id).await
}

/// US-05: monitors, incidents and tickets stay with the project. The new owner is an active TL
/// or the super admin making the change; anyone else is 404.
pub async fn reassign(
    db: &Db,
    project_id: i64,
    new_owner: i64,
    acting: &User,
) -> Result<(), PageError> {
    let done = db
        .write(async |c| {
            if new_owner != acting.id && !crate::accounts::is_active_tl(c, new_owner).await? {
                return Ok(false);
            }
            store::set_owner(c, project_id, new_owner, now()).await?;
            Ok(true)
        })
        .await?;
    if done {
        Ok(())
    } else {
        Err(PageError::NotFound)
    }
}

/// US-07: returns the raw key once. Only its hash is stored.
pub async fn create_key(db: &Db, project_id: i64, name: &str) -> Result<String, ProjectError> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 60 {
        return Err(ProjectError::InvalidName);
    }
    let raw = format!("mk_{}", token::random_token());
    let hash = token::hash(&raw);
    let prefix: String = raw.chars().take(9).collect();
    db.write(async |c| store::insert_key(c, project_id, name, &hash, &prefix).await)
        .await?;
    Ok(raw)
}

pub async fn list_keys(db: &Db, project_id: i64) -> anyhow::Result<Vec<ApiKeyRow>> {
    store::list_keys(&db.read().await?, project_id).await
}

pub async fn revoke_key(db: &Db, project_id: i64, key_id: i64) -> anyhow::Result<()> {
    db.write(async |c| store::revoke_key(c, project_id, key_id, now()).await)
        .await
}

/// US-08: an empty value turns the allowlist off.
pub async fn set_allowlist(db: &Db, project_id: i64, raw: &str) -> Result<(), ProjectError> {
    let value = allowlist::normalize(raw).map_err(ProjectError::InvalidCidr)?;
    db.write(async |c| store::set_allowlist(c, project_id, value.as_deref(), now()).await)
        .await?;
    Ok(())
}

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
        let conn = state.db.read().await.map_err(internal)?;
        let (key_id, project, last_used) = store::find_live_key(&conn, &token::hash(raw.trim()))
            .await
            .map_err(internal)?
            .ok_or_else(unauthorized)?;
        if let Some(list) = &project.ip_allowlist {
            let (headers, extensions) = (&parts.headers, &parts.extensions);
            let ip = crate::platform::caller_ip(headers, extensions, state.config.trust_proxy);
            if !ip.is_some_and(|ip| allowlist::contains(list, ip)) {
                return Err(api_error(
                    StatusCode::FORBIDDEN,
                    "forbidden",
                    "This IP may not use this key.",
                ));
            }
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
