//! tickets module: ref IDs, tickets, kanban, public board.
//! Spec: docs/technical-specs/05-module-definitions.md section 5.7.

pub mod routes;

use crate::platform::{db::Db, token};

/// US-23: returns the raw ref ID once. Only its hash is stored.
pub async fn issue_ref_id(
    db: &Db,
    project_id: i64,
    single_use: bool,
) -> anyhow::Result<(String, i64)> {
    let raw = format!("rf_{}", token::random_token());
    let hash = token::hash(&raw);
    let created_at = db
        .write(async |c| {
            c.execute(
                "INSERT INTO ref_ids (project_id, token_hash, single_use) VALUES (?1, ?2, ?3)",
                (project_id, hash.as_str(), single_use),
            )
            .await?;
            Ok(crate::platform::now())
        })
        .await?;
    Ok((raw, created_at))
}
