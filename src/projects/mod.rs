//! projects module: projects, ownership, API keys, IP allowlist.
//! Spec: docs/technical-specs/05-module-definitions.md section 5.2.

mod allowlist;
pub mod api;
mod icon;
mod overview;
mod profile;
mod pulse;
pub mod routes;
pub mod secrets;
mod settings_api;
mod store;

use axum::{
    extract::FromRequestParts,
    http::{StatusCode, header, request::Parts},
    response::Response,
};

use crate::{
    accounts::User,
    app::AppState,
    platform::{api_error, caps, db::Db, now, page::PageError, token},
};

pub use profile::{ProfileInput, ProjectInput, create, find_owner, icon, slug_for, update_profile};
pub use store::{ApiKeyRow, Project, Sort};

#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    #[error("Enter a name.")]
    InvalidName,
    #[error("Pick the owner from the list: yourself or an active tech lead.")]
    UnknownOwner,
    #[error("The description needs at most 280 characters.")]
    LongDescription,
    /// Bad values in several fields of one form, in the form's order.
    #[error("{}", .0.first().map_or("Fix the fields marked below.", |f| f.1))]
    Invalid(Vec<(&'static str, &'static str)>),
    #[error("{0}")]
    BadIcon(&'static str),
    #[error("Not a valid IP or CIDR: {0}")]
    InvalidCidr(String),
    #[error("Type the project's name exactly to delete it.")]
    ConfirmMismatch,
    /// A hard cap in `platform::caps` is reached.
    #[error("{0}")]
    Full(String),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// One page of the projects a user may open, and how many match in all.
pub struct ProjectPage {
    pub projects: Vec<Project>,
    pub total: i64,
    pub page: i64,
}

/// A tech lead pages through their own projects; a super admin through every project.
/// `page` counts from 1; a lower value reads page 1. `q` matches name, slug, owner email or
/// description, ignoring ASCII case. `owner` is an owner's email, or empty for any.
pub async fn page_for(
    db: &Db,
    user: &User,
    q: &str,
    owner: &str,
    sort: Sort,
    page: i64,
) -> anyhow::Result<ProjectPage> {
    let q: String = q.trim().to_lowercase().chars().take(100).collect();
    let f = store::Filter {
        owner_id: (!user.sees_all()).then_some(user.id),
        q: &q,
        owner_email: owner.trim(),
        sort,
    };
    let page = page.max(1);
    let offset = (page - 1).saturating_mul(caps::PROJECTS_PAGE);
    let conn = db.read().await?;
    Ok(ProjectPage {
        total: store::count_matching(&conn, &f).await?,
        projects: store::page_matching(&conn, &f, caps::PROJECTS_PAGE, offset).await?,
        page,
    })
}

/// Emails that own a project, for a super admin's owner filter.
pub async fn owner_emails(db: &Db) -> anyhow::Result<Vec<String>> {
    store::owner_emails(&db.read().await?).await
}

/// Projects in id order after `last_id`, at most `limit`, for a job that walks every project
/// inside its own transaction. Loop until a batch comes back short.
pub async fn after_id(
    c: &turso::Connection,
    last_id: i64,
    limit: i64,
) -> anyhow::Result<Vec<Project>> {
    store::after_id(c, last_id, limit).await
}

pub use pulse::{Pulse, pulse};

/// The public tabs a project shows besides Uptime, which always shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PublicPages {
    /// Missing means on, so a client that never sends it keeps the tab.
    #[serde(default = "on")]
    pub incidents: bool,
    pub known_issues: bool,
    /// The Report a bug tab: the form and the reporter's own tickets, both behind a ref ID.
    pub report: bool,
    /// The Changelog tab. Missing means off: it starts off on every project.
    #[serde(default)]
    pub changelog: bool,
}

fn on() -> bool {
    true
}

/// A project's numbers. The tabs show only the ones that flag a problem: monitors and heartbeats
/// that are down, open incidents, and tickets waiting in Triage. The About column shows totals.
#[derive(Default, serde::Serialize)]
pub struct TabCounts {
    pub monitors: i64,
    pub heartbeats: i64,
    pub monitors_down: i64,
    pub heartbeats_down: i64,
    pub incidents: i64,
    pub tickets: i64,
    pub triage: i64,
}

/// Each number read through its module's interface.
pub async fn tab_counts(db: &Db, project_id: i64) -> anyhow::Result<TabCounts> {
    use crate::incidents::{ItemState, SourceKind};
    let (monitors, heartbeats) = crate::checks::counts(db, project_id).await?;
    let states = crate::incidents::team_states_for(db, project_id).await?;
    let down = |kind: SourceKind| {
        states
            .iter()
            .filter(|((k, _), s)| *k == kind && **s == ItemState::Down)
            .count() as i64
    };
    let triage = crate::tickets::stage_counts(db, project_id)
        .await?
        .into_iter()
        .find(|(stage, _)| stage == "triage")
        .map_or(0, |(_, n)| n);
    Ok(TabCounts {
        monitors,
        heartbeats,
        monitors_down: down(SourceKind::Monitor),
        heartbeats_down: down(SourceKind::Heartbeat),
        incidents: crate::incidents::open_count(db, project_id).await?,
        tickets: crate::tickets::open_count(db, project_id).await?,
        triage,
    })
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

/// Runs inside the caller's write: a user who owns a project cannot become a supervisor, since
/// an owner must be able to change their project.
pub async fn owns_any(c: &turso::Connection, user_id: i64) -> anyhow::Result<bool> {
    store::owns_any(c, user_id).await
}

/// A super admin and a supervisor see every project; a tech lead sees their own. A supervisor's
/// writes never get this far: `CurrentUser` refuses them.
pub fn can_view(user: &User, project: &Project) -> bool {
    user.sees_all() || project.owner_id == user.id
}

/// The project a user may open, or 404 so other projects' names do not leak.
pub async fn find_for(db: &Db, user: &User, slug: &str) -> Result<Project, PageError> {
    let project = store::find_by_slug(&db.read().await?, slug).await?;
    match project {
        Some(p) if can_view(user, &p) => Ok(p),
        _ => Err(PageError::NotFound),
    }
}

/// The projects a cross-project page names, keyed by id. Ids come from rows the caller may see.
pub async fn by_ids(
    db: &Db,
    ids: &[i64],
) -> anyhow::Result<std::collections::HashMap<i64, Project>> {
    Ok(store::by_ids(&db.read().await?, ids)
        .await?
        .into_iter()
        .map(|p| (p.id, p))
        .collect())
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
    db.write(async |c| {
        if store::count_live_keys(c, project_id).await? >= caps::KEYS_PER_PROJECT {
            return Ok(Err(ProjectError::Full(format!(
                "A project holds {} live keys at most. Revoke one first.",
                caps::KEYS_PER_PROJECT
            ))));
        }
        store::insert_key(c, project_id, name, &hash, &prefix).await?;
        Ok(Ok(()))
    })
    .await??;
    Ok(raw)
}

pub async fn list_keys(db: &Db, project_id: i64) -> anyhow::Result<Vec<ApiKeyRow>> {
    store::list_keys(&db.read().await?, project_id).await
}

pub async fn revoke_key(db: &Db, project_id: i64, key_id: i64) -> anyhow::Result<()> {
    db.write(async |c| store::revoke_key(c, project_id, key_id, now()).await)
        .await
}

/// Which public tabs visitors see besides Status.
pub async fn set_pages(db: &Db, project_id: i64, pages: PublicPages) -> anyhow::Result<()> {
    db.write(async |c| store::set_pages(c, project_id, pages, now()).await)
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
            let ip = crate::platform::caller_ip(
                &parts.headers,
                &parts.extensions,
                state.config.trust_proxy,
            );
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

/// A super admin deletes a project and everything in it. `confirm` must be the project's name.
pub async fn delete(db: &Db, project: &Project, confirm: &str) -> Result<(), ProjectError> {
    if confirm.trim() != project.name {
        return Err(ProjectError::ConfirmMismatch);
    }
    db.write(async |c| {
        let sources = crate::checks::forget_project(c, project.id).await?;
        crate::incidents::forget_sources(c, &sources).await?;
        store::delete_project(c, project.id).await
    })
    .await?;
    Ok(())
}
