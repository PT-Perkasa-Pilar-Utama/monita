//! SQL for changelog_entries. Newest first everywhere: `released_on DESC, id DESC`.

use serde::Serialize;
use turso::Connection;

use super::{ReleaseInput, Section};
use crate::platform::db::{query_all, query_i64};

/// A release as the team edits it and the public card shows it.
#[derive(Serialize)]
pub struct Release {
    pub id: i64,
    pub version: String,
    pub released_on: String,
    pub title: String,
    pub sections: Vec<Section>,
}

/// A release in the public index: what the visitor needs to pick it.
#[derive(Serialize)]
pub struct ReleaseHead {
    #[serde(skip)]
    pub id: i64,
    pub version: String,
    pub title: String,
    pub released_on: String,
}

const COLS: &str = "id, version, released_on, title, sections";

fn release(r: &turso::Row) -> turso::Result<Release> {
    let raw: String = r.get(4)?;
    Ok(Release {
        id: r.get(0)?,
        version: r.get(1)?,
        released_on: r.get(2)?,
        title: r.get(3)?,
        // Written by `insert` and `update` from validated input; a row that fails shows empty.
        sections: serde_json::from_str(&raw).unwrap_or_default(),
    })
}

fn head(r: &turso::Row) -> turso::Result<ReleaseHead> {
    Ok(ReleaseHead {
        id: r.get(0)?,
        version: r.get(1)?,
        title: r.get(2)?,
        released_on: r.get(3)?,
    })
}

pub async fn page(
    c: &Connection,
    project_id: i64,
    limit: i64,
    offset: i64,
) -> anyhow::Result<Vec<Release>> {
    query_all(
        c,
        format!("SELECT {COLS} FROM changelog_entries WHERE project_id = ?1 ORDER BY released_on DESC, id DESC LIMIT ?2 OFFSET ?3"),
        (project_id, limit, offset),
        release,
    )
    .await
}

pub async fn count(c: &Connection, project_id: i64) -> anyhow::Result<i64> {
    Ok(query_i64(
        c,
        "SELECT count(*) FROM changelog_entries WHERE project_id = ?1",
        [project_id],
    )
    .await?
    .unwrap_or(0))
}

/// The id of the project's release with this version.
pub async fn find_version(
    c: &Connection,
    project_id: i64,
    version: &str,
) -> anyhow::Result<Option<i64>> {
    query_i64(
        c,
        "SELECT id FROM changelog_entries WHERE project_id = ?1 AND version = ?2",
        (project_id, version),
    )
    .await
}

fn sections_json(sections: &[Section]) -> anyhow::Result<String> {
    Ok(serde_json::to_string(sections)?)
}

pub async fn insert(
    c: &Connection,
    project_id: i64,
    r: &ReleaseInput,
    sections: &[Section],
    at: i64,
) -> anyhow::Result<i64> {
    c.execute(
        "INSERT INTO changelog_entries (project_id, version, released_on, title, sections, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
        (project_id, r.version.trim(), r.released_on.trim(), r.title.trim(), sections_json(sections)?, at),
    )
    .await?;
    Ok(c.last_insert_rowid())
}

pub async fn update(
    c: &Connection,
    project_id: i64,
    id: i64,
    r: &ReleaseInput,
    sections: &[Section],
    at: i64,
) -> anyhow::Result<bool> {
    let n = c
        .execute(
            "UPDATE changelog_entries SET version = ?1, released_on = ?2, title = ?3, sections = ?4, updated_at = ?5
             WHERE id = ?6 AND project_id = ?7",
            (r.version.trim(), r.released_on.trim(), r.title.trim(), sections_json(sections)?, at, id, project_id),
        )
        .await?;
    Ok(n > 0)
}

pub async fn delete(c: &Connection, project_id: i64, id: i64) -> anyhow::Result<()> {
    c.execute(
        "DELETE FROM changelog_entries WHERE id = ?1 AND project_id = ?2",
        (id, project_id),
    )
    .await?;
    Ok(())
}

/// One page of the index, older than the `(released_on, id)` cursor when there is one.
pub async fn index(
    c: &Connection,
    project_id: i64,
    before: Option<(String, i64)>,
    limit: i64,
) -> anyhow::Result<Vec<ReleaseHead>> {
    let (day, id) = before.unwrap_or_else(|| ("9999-12-31".into(), i64::MAX));
    query_all(
        c,
        "SELECT id, version, title, released_on FROM changelog_entries
         WHERE project_id = ?1 AND (released_on < ?2 OR (released_on = ?2 AND id < ?3))
         ORDER BY released_on DESC, id DESC LIMIT ?4",
        (project_id, day, id, limit),
        head,
    )
    .await
}

pub async fn by_version(
    c: &Connection,
    project_id: i64,
    version: &str,
) -> anyhow::Result<Option<Release>> {
    Ok(query_all(
        c,
        format!("SELECT {COLS} FROM changelog_entries WHERE project_id = ?1 AND version = ?2"),
        (project_id, version),
        release,
    )
    .await?
    .pop())
}

pub async fn newest(c: &Connection, project_id: i64) -> anyhow::Result<Option<Release>> {
    Ok(query_all(
        c,
        format!("SELECT {COLS} FROM changelog_entries WHERE project_id = ?1 ORDER BY released_on DESC, id DESC LIMIT 1"),
        [project_id],
        release,
    )
    .await?
    .pop())
}

/// The release just newer and just older than `r`.
pub async fn neighbours(
    c: &Connection,
    project_id: i64,
    r: &Release,
) -> anyhow::Result<(Option<ReleaseHead>, Option<ReleaseHead>)> {
    let newer = query_all(
        c,
        "SELECT id, version, title, released_on FROM changelog_entries
         WHERE project_id = ?1 AND (released_on > ?2 OR (released_on = ?2 AND id > ?3))
         ORDER BY released_on ASC, id ASC LIMIT 1",
        (project_id, r.released_on.as_str(), r.id),
        head,
    )
    .await?
    .pop();
    let older = index(c, project_id, Some((r.released_on.clone(), r.id)), 1)
        .await?
        .pop();
    Ok((newer, older))
}
