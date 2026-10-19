//! What a super admin does to other accounts: change a role, disable, enable, reset sign-in.
//! Nobody acts on their own account here, so at least one active super admin always remains.

use super::{Role, store};
use crate::platform::{db::Db, now};

#[derive(Debug, thiserror::Error)]
pub enum AdminError {
    #[error("You cannot change your own account here.")]
    Yourself,
    #[error("No such user, or the change does not apply to them.")]
    NotFound,
    #[error("This person owns projects. Give their projects to another owner first.")]
    OwnsProjects,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

fn not_self(actor: i64, user_id: i64) -> Result<(), AdminError> {
    if actor == user_id {
        Err(AdminError::Yourself)
    } else {
        Ok(())
    }
}

fn found(changed: bool) -> Result<(), AdminError> {
    if changed {
        Ok(())
    } else {
        Err(AdminError::NotFound)
    }
}

/// Makes an active user a super admin, a tech lead or a supervisor. The role is read on every
/// request, so the change holds from their next click. A project owner cannot become a
/// supervisor: an owner must be able to change their project.
pub async fn set_role(db: &Db, actor: i64, user_id: i64, role: Role) -> Result<(), AdminError> {
    not_self(actor, user_id)?;
    let at = now();
    let changed = db
        .write(async |c| {
            if role == Role::Supervisor && crate::projects::owns_any(c, user_id).await? {
                return Ok(Err(AdminError::OwnsProjects));
            }
            Ok(Ok(store::set_role(c, user_id, role, at).await?))
        })
        .await??;
    found(changed)
}

/// US-04: signs them out at once and drops their invites. Their projects stay theirs.
pub async fn disable(db: &Db, actor: i64, user_id: i64) -> Result<(), AdminError> {
    not_self(actor, user_id)?;
    let at = now();
    found(
        db.write(async |c| store::disable_user(c, user_id, at).await)
            .await?,
    )
}

/// Undoes a disable. A user who had set a password signs in with it again; one who had not
/// needs a new invite, since disabling dropped theirs.
pub async fn enable(db: &Db, user_id: i64) -> Result<(), AdminError> {
    let at = now();
    found(
        db.write(async |c| store::enable_user(c, user_id, at).await)
            .await?,
    )
}

/// A new invite for someone else; it resets their sign-in. Returns their email and the link.
pub async fn reset_sign_in(
    db: &Db,
    base_url: &str,
    actor: i64,
    user_id: i64,
) -> Result<(String, String), AdminError> {
    not_self(actor, user_id)?;
    super::reinvite(db, base_url, user_id)
        .await
        .map_err(|e| match e {
            crate::platform::page::PageError::Internal(e) => AdminError::Other(e),
            _ => AdminError::NotFound,
        })
}
