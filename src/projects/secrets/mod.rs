//! Sealed secrets: a project's tokens and passwords, written once and never shown again.
//! A value is stored as entered, because a check must send it; no response carries it.
// ponytail: stored in plain text in the database file, like the mail password. Upgrade path:
// encrypt with aws-lc-rs AEAD (already built in) under a key file beside the database.

pub mod api;
mod store;

use serde::Serialize;
use turso::Connection;

use crate::platform::{caps, db::Db, now};

#[derive(Clone, Debug, Serialize)]
pub struct SecretRow {
    pub id: i64,
    pub name: String,
    pub updated_at: i64,
    /// Monitors that authenticate with it.
    pub used_by: i64,
}

#[derive(Debug, thiserror::Error)]
pub enum SecretError {
    #[error("Use 1 to 60 letters, digits, _ or -, for example REPORT_API_TOKEN.")]
    BadName,
    #[error("Another secret has this name.")]
    NameTaken,
    #[error("Enter a value of up to 4 KB.")]
    BadValue,
    #[error(
        "A project holds {} secrets at most, so delete one first.",
        caps::SECRETS_PER_PROJECT
    )]
    Full,
    #[error("{0} use it, so change them first.")]
    InUse(String),
    #[error("This secret does not exist.")]
    NotFound,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

fn valid_name(name: &str) -> bool {
    (1..=60).contains(&name.chars().count())
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
}

fn valid_value(value: &str) -> bool {
    !value.is_empty() && value.len() <= caps::SECRET_VALUE_MAX
}

/// A project's secrets by name, with how many monitors use each. Never the values.
pub async fn list(db: &Db, project_id: i64) -> anyhow::Result<Vec<SecretRow>> {
    let c = db.read().await?;
    let mut out = Vec::new();
    for (id, name, updated_at) in store::list(&c, project_id).await? {
        let used_by = crate::checks::secret_users(&c, id).await?;
        out.push(SecretRow {
            id,
            name,
            updated_at,
            used_by,
        });
    }
    Ok(out)
}

pub async fn create(db: &Db, project_id: i64, name: &str, value: &str) -> Result<i64, SecretError> {
    let name = name.trim();
    if !valid_name(name) {
        return Err(SecretError::BadName);
    }
    if !valid_value(value) {
        return Err(SecretError::BadValue);
    }
    let at = now();
    db.write(async |c| {
        if store::count(c, project_id).await? >= caps::SECRETS_PER_PROJECT {
            return Ok(Err(SecretError::Full));
        }
        if store::name_taken(c, project_id, name).await? {
            return Ok(Err(SecretError::NameTaken));
        }
        Ok(Ok(store::insert(c, project_id, name, value, at).await?))
    })
    .await?
}

pub async fn replace(db: &Db, project_id: i64, id: i64, value: &str) -> Result<(), SecretError> {
    if !valid_value(value) {
        return Err(SecretError::BadValue);
    }
    let at = now();
    let found = db
        .write(async |c| store::replace(c, project_id, id, value, at).await)
        .await?;
    if found {
        Ok(())
    } else {
        Err(SecretError::NotFound)
    }
}

/// Refused while a monitor uses it, so no check silently loses its credentials.
pub async fn delete(db: &Db, project_id: i64, id: i64) -> Result<(), SecretError> {
    db.write(async |c| {
        let n = crate::checks::secret_users(c, id).await?;
        if n > 0 {
            let who = if n == 1 {
                "1 monitor".to_string()
            } else {
                format!("{n} monitors")
            };
            return Ok(Err(SecretError::InUse(who)));
        }
        Ok(if store::delete(c, project_id, id).await? {
            Ok(())
        } else {
            Err(SecretError::NotFound)
        })
    })
    .await?
}

/// Whether `id` is a secret of this project, for a monitor form.
pub async fn exists(c: &Connection, project_id: i64, id: i64) -> anyhow::Result<bool> {
    Ok(store::find(c, project_id, id).await?.is_some())
}

/// The name of a project's secret, for a monitor's view.
pub async fn name_of(c: &Connection, project_id: i64, id: i64) -> anyhow::Result<Option<String>> {
    Ok(store::find(c, project_id, id).await?.map(|s| s.name))
}

/// The value a check sends. The only door to a secret's value.
pub async fn value_of(c: &Connection, project_id: i64, id: i64) -> anyhow::Result<Option<String>> {
    store::value_of(c, project_id, id).await
}

#[cfg(test)]
mod tests {
    #[test]
    fn names_are_short_words_of_letters_digits_and_dashes() {
        assert!(super::valid_name("REPORT_API_TOKEN"));
        assert!(super::valid_name("svc-1"));
        assert!(!super::valid_name(""));
        assert!(!super::valid_name("has space"));
        assert!(!super::valid_name(&"x".repeat(61)));
        assert!(!super::valid_value(""));
        assert!(!super::valid_value(&"x".repeat(4097)));
    }
}
