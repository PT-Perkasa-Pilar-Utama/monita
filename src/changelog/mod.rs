//! changelog module: a project's releases, written by its team and shown on the public Changelog
//! tab one at a time, newest first.

pub mod api;
mod store;

use serde::{Deserialize, Serialize};

use crate::platform::{caps, db::Db, now};

pub use store::{Release, ReleaseHead};

/// The kinds of change a release lists, in the order a release shows them.
pub const TAGS: [&str; 4] = ["Added", "Changed", "Fixed", "Removed"];
const TITLE_MAX: usize = 120;
const ITEMS_MAX: usize = 30;
const ITEM_MAX: usize = 500;

/// One kind of change and its lines.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Section {
    pub tag: String,
    pub items: Vec<String>,
}

/// The release form: one change per line in each of the four boxes.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct ReleaseInput {
    pub version: String,
    /// `YYYY-MM-DD`.
    pub released_on: String,
    pub title: String,
    pub added: String,
    pub changed: String,
    pub fixed: String,
    pub removed: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ChangelogError {
    /// Every bad field of the form, in the form's order.
    #[error("{}", .0.first().map_or("Fix the fields marked below.", |f| f.1))]
    Invalid(Vec<(&'static str, &'static str)>),
    #[error("This project already has a release with this version.")]
    Taken,
    #[error("This release does not exist.")]
    NotFound,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// `latest` is taken: the public route names the newest release with it.
fn valid_version(v: &str) -> bool {
    let mut chars = v.chars();
    v != "latest"
        && (1..=40).contains(&v.len())
        && chars.next().is_some_and(|c| c.is_ascii_alphanumeric())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

/// `YYYY-MM-DD` with a real month and day.
fn valid_day(d: &str) -> bool {
    let b = d.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return false;
    }
    let num = |r: std::ops::Range<usize>| d.get(r).and_then(|s| s.parse::<u32>().ok());
    matches!(
        (num(0..4), num(5..7), num(8..10)),
        (Some(1970..=9999), Some(1..=12), Some(1..=31))
    )
}

impl ReleaseInput {
    /// The sections to store, or every bad field at once.
    pub fn validate(&self) -> Result<Vec<Section>, ChangelogError> {
        let mut bad = Vec::new();
        if !valid_version(self.version.trim()) {
            bad.push((
                "version",
                "Use 1 to 40 letters, digits, dots, dashes or underscores, starting with a letter or digit.",
            ));
        }
        if !valid_day(self.released_on.trim()) {
            bad.push(("released_on", "Pick the day of the release."));
        }
        let title = self.title.trim().chars().count();
        if title == 0 || title > TITLE_MAX {
            bad.push(("title", "Enter a title of up to 120 characters."));
        }
        let boxes = [&self.added, &self.changed, &self.fixed, &self.removed];
        let fields = ["added", "changed", "fixed", "removed"];
        let mut sections = Vec::new();
        for ((tag, field), text) in TAGS.iter().zip(fields).zip(boxes) {
            let items: Vec<String> = text
                .lines()
                .map(|l| l.trim().trim_start_matches(['-', '*']).trim())
                .filter(|l| !l.is_empty())
                .map(str::to_string)
                .collect();
            if items.len() > ITEMS_MAX || items.iter().any(|i| i.chars().count() > ITEM_MAX) {
                bad.push((field, "Use at most 30 lines of up to 500 characters."));
            }
            if !items.is_empty() {
                sections.push(Section {
                    tag: (*tag).to_string(),
                    items,
                });
            }
        }
        if sections.is_empty() && !bad.iter().any(|(f, _)| fields.contains(f)) {
            bad.push((
                "added",
                "Write at least one change in any of the four boxes.",
            ));
        }
        if bad.is_empty() {
            Ok(sections)
        } else {
            Err(ChangelogError::Invalid(bad))
        }
    }
}

/// One page of the team's list matching `q` (version or title), newest first unless `sort` is
/// `oldest`, and how many match.
pub async fn page(
    db: &Db,
    project_id: i64,
    q: &str,
    sort: &str,
    page: i64,
) -> anyhow::Result<(Vec<Release>, i64)> {
    let c = db.read().await?;
    let offset = (page.max(1) - 1).saturating_mul(caps::CHANGELOG_PAGE);
    Ok((
        store::page(&c, project_id, q, sort, caps::CHANGELOG_PAGE, offset).await?,
        store::count(&c, project_id, q).await?,
    ))
}

pub async fn create(db: &Db, project_id: i64, input: &ReleaseInput) -> Result<i64, ChangelogError> {
    let sections = input.validate()?;
    db.write(async |c| {
        if store::find_version(c, project_id, input.version.trim())
            .await?
            .is_some()
        {
            return Ok(Err(ChangelogError::Taken));
        }
        Ok(Ok(
            store::insert(c, project_id, input, &sections, now()).await?
        ))
    })
    .await?
}

pub async fn update(
    db: &Db,
    project_id: i64,
    id: i64,
    input: &ReleaseInput,
) -> Result<(), ChangelogError> {
    let sections = input.validate()?;
    db.write(async |c| {
        if store::find_version(c, project_id, input.version.trim())
            .await?
            .is_some_and(|other| other != id)
        {
            return Ok(Err(ChangelogError::Taken));
        }
        Ok(
            if store::update(c, project_id, id, input, &sections, now()).await? {
                Ok(())
            } else {
                Err(ChangelogError::NotFound)
            },
        )
    })
    .await?
}

pub async fn delete(db: &Db, project_id: i64, id: i64) -> anyhow::Result<()> {
    db.write(async |c| store::delete(c, project_id, id).await)
        .await
}

/// The public index: version, title and day of one page of releases matching `q`, older than the
/// `(released_on, id)` cursor, newest first.
pub async fn index(
    db: &Db,
    project_id: i64,
    q: &str,
    before: Option<(String, i64)>,
) -> anyhow::Result<Vec<ReleaseHead>> {
    store::index(
        &db.read().await?,
        project_id,
        q,
        before,
        caps::CHANGELOG_PAGE,
    )
    .await
}

/// One release by version, with its newer and older neighbours' versions; the newest when
/// `version` is `None`.
pub async fn one(
    db: &Db,
    project_id: i64,
    version: Option<&str>,
) -> anyhow::Result<Option<(Release, Option<ReleaseHead>, Option<ReleaseHead>)>> {
    let c = db.read().await?;
    let found = match version {
        Some(v) => store::by_version(&c, project_id, v).await?,
        None => store::newest(&c, project_id).await?,
    };
    let Some(r) = found else {
        return Ok(None);
    };
    let (newer, older) = store::neighbours(&c, project_id, &r).await?;
    Ok(Some((r, newer, older)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> ReleaseInput {
        ReleaseInput {
            version: "1.4.0".into(),
            released_on: "2026-10-19".into(),
            title: "Faster exports".into(),
            added: "- CSV export\n\n* PDF export ".into(),
            ..ReleaseInput::default()
        }
    }

    #[test]
    fn lines_become_items_without_their_bullets() {
        let s = input().validate().unwrap();
        assert_eq!(
            s,
            [Section {
                tag: "Added".into(),
                items: vec!["CSV export".into(), "PDF export".into()]
            }]
        );
    }

    #[test]
    fn a_bad_form_names_every_bad_field() {
        let bad = ReleaseInput {
            version: "../x".into(),
            released_on: "2026-13-01".into(),
            ..ReleaseInput::default()
        };
        let Err(ChangelogError::Invalid(fields)) = bad.validate() else {
            panic!("expected field errors");
        };
        let names: Vec<_> = fields.iter().map(|f| f.0).collect();
        assert_eq!(names, ["version", "released_on", "title", "added"]);
    }
}
