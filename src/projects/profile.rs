//! A project's own fields: its name, owner and slug at creation, then its description and icon.

use serde::Deserialize;

use super::{Project, ProjectError, eligible_owners, icon::parse as parse_icon, store};
use crate::{
    accounts::User,
    platform::{db::Db, now, token},
};

/// The random part of a generated slug: 6 base32 characters, about a billion values.
const SUFFIX: usize = 6;
/// The longest slug the data model allows (technical-specs 06).
const SLUG_MAX: usize = 40;
/// The longest description, in characters.
const DESCRIPTION_MAX: usize = 280;

/// The New project form. `owner_email` counts only when a super admin sends it.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct ProjectInput {
    pub name: String,
    pub owner_email: String,
    pub description: String,
    /// Where people use the system. Required.
    pub homepage_url: String,
    /// Base64 of the scaled icon, filled by the page's script; empty for none.
    pub icon: String,
}

/// The Edit project form on Settings, General.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct ProfileInput {
    pub description: String,
    pub homepage_url: String,
    pub icon: String,
    /// A checkbox: "on" removes the icon when no new one is sent.
    pub remove_icon: String,
}

const HOMEPAGE_HINT: &str =
    "Enter the address where people use it, starting with https://, with no user name or password.";

/// The homepage, trimmed, or the error for its field. A URL of up to 2 000 characters.
fn homepage(raw: &str) -> Option<&str> {
    let u = raw.trim();
    (crate::platform::valid_http_url(u) && u.len() <= 2_000 && !crate::platform::has_userinfo(u))
        .then_some(u)
}

/// An empty description is none.
fn description(raw: &str) -> Result<Option<&str>, ProjectError> {
    match raw.trim() {
        "" => Ok(None),
        d if d.chars().count() > DESCRIPTION_MAX => Err(ProjectError::LongDescription),
        d => Ok(Some(d)),
    }
}

/// US-06: the name in lowercase letters, digits and dashes, cut to fit, then `-` and the suffix.
/// A name with none of those characters becomes `project`.
pub fn slug_for(name: &str, suffix: &str) -> String {
    let mut base = String::new();
    for c in name.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            base.push(c);
        } else if !base.is_empty() && !base.ends_with('-') {
            base.push('-');
        }
    }
    let base: String = base.chars().take(SLUG_MAX - 1 - suffix.len()).collect();
    let base = match base.trim_end_matches('-') {
        "" => "project",
        b => b,
    };
    format!("{base}-{suffix}")
}

/// The user a super admin names by email: the admin or an active tech lead. Case is ignored.
pub async fn find_owner(db: &Db, acting: &User, email: &str) -> anyhow::Result<Option<i64>> {
    let email = email.trim();
    Ok(eligible_owners(db, acting)
        .await?
        .into_iter()
        .find(|u| u.email.eq_ignore_ascii_case(email))
        .map(|u| u.id))
}

/// US-06: creates the project, plus its two alert routes. A tech lead owns what they create; a
/// super admin owns it too unless the form names another owner.
pub async fn create(db: &Db, acting: &User, input: &ProjectInput) -> Result<Project, ProjectError> {
    let name = input.name.trim();
    let mut bad = Vec::new();
    if name.is_empty() || name.chars().count() > 80 {
        bad.push(("name", "Enter a name of up to 80 characters."));
    }
    let homepage_url = homepage(&input.homepage_url);
    if homepage_url.is_none() {
        bad.push(("homepage_url", HOMEPAGE_HINT));
    }
    let description = description(&input.description);
    if description.is_err() {
        bad.push((
            "description",
            "The description needs at most 280 characters.",
        ));
    }
    let icon = parse_icon(&input.icon);
    if let Err(ProjectError::BadIcon(m)) = &icon {
        bad.push(("icon", m));
    }
    let (Some(homepage_url), Ok(description), Ok(icon), true) =
        (homepage_url, description, icon, bad.is_empty())
    else {
        return Err(ProjectError::Invalid(bad));
    };
    let owner = if acting.is_admin() && !input.owner_email.trim().is_empty() {
        find_owner(db, acting, &input.owner_email)
            .await?
            .ok_or(ProjectError::UnknownOwner)?
    } else {
        acting.id
    };
    let slug = db
        .write(async |c| {
            if owner != acting.id && !crate::accounts::is_active_tl(c, owner).await? {
                return Ok(Err(ProjectError::UnknownOwner));
            }
            if owner == acting.id && !crate::accounts::can_own_projects(c, owner).await? {
                return Ok(Err(ProjectError::CannotOwn));
            }
            // A taken slug needs the same name and the same suffix; three draws in a row is not
            // a real case, so it is an error rather than a loop.
            for _ in 0..3 {
                let slug = slug_for(name, &token::random_token()[..SUFFIX]);
                if store::find_by_slug(c, &slug).await?.is_none() {
                    let id =
                        store::insert(c, owner, name, &slug, description, homepage_url).await?;
                    if let Some(i) = &icon {
                        store::set_icon(c, id, i.mime, i.data.clone(), now()).await?;
                    }
                    crate::notify::create_routes(c, id, owner).await?;
                    return Ok(Ok(slug));
                }
            }
            Err(anyhow::anyhow!(
                "three random slugs for {name:?} were taken"
            ))
        })
        .await??;
    store::find_by_slug(&db.read().await?, &slug)
        .await?
        .ok_or_else(|| ProjectError::Other(anyhow::anyhow!("project {slug} vanished after insert")))
}

/// Saves the description, and sets, keeps or removes the icon. A new icon wins over remove.
pub async fn update_profile(
    db: &Db,
    project_id: i64,
    input: &ProfileInput,
) -> Result<(), ProjectError> {
    let description = description(&input.description);
    let homepage_url = homepage(&input.homepage_url);
    let mut bad = Vec::new();
    if description.is_err() {
        bad.push((
            "description",
            "The description needs at most 280 characters.",
        ));
    }
    if homepage_url.is_none() {
        bad.push(("homepage_url", HOMEPAGE_HINT));
    }
    let icon = parse_icon(&input.icon);
    if let Err(ProjectError::BadIcon(m)) = &icon {
        bad.push(("icon", m));
    }
    let (Ok(description), Some(homepage_url), Ok(icon), true) =
        (description, homepage_url, icon, bad.is_empty())
    else {
        return Err(ProjectError::Invalid(bad));
    };
    let remove = input.remove_icon == "on";
    db.write(async |c| {
        let at = now();
        store::set_description(c, project_id, description, homepage_url, at).await?;
        match &icon {
            Some(i) => store::set_icon(c, project_id, i.mime, i.data.clone(), at).await?,
            None if remove => store::remove_icon(c, project_id, at).await?,
            None => {}
        }
        Ok(())
    })
    .await?;
    Ok(())
}

/// The icon's type and bytes, for its own route.
pub async fn icon(db: &Db, project_id: i64) -> anyhow::Result<Option<(String, Vec<u8>)>> {
    store::icon(&db.read().await?, project_id).await
}

#[cfg(test)]
mod tests {
    use super::slug_for;

    #[test]
    fn a_slug_is_the_name_in_lowercase_with_dashes_then_the_suffix() {
        assert_eq!(
            slug_for("  Customer Portal (EU)! ", "ab2cd3"),
            "customer-portal-eu-ab2cd3"
        );
        assert_eq!(slug_for("Kasir ☕ Toko", "ab2cd3"), "kasir-toko-ab2cd3");
    }

    #[test]
    fn a_long_name_is_cut_to_fit_and_a_name_without_letters_becomes_project() {
        let slug = slug_for(&"a-".repeat(40), "ab2cd3");
        assert!(slug.len() <= 40 && !slug.contains("--"), "{slug}");
        assert_eq!(slug_for("☕☕", "ab2cd3"), "project-ab2cd3");
    }
}
