//! US-30: the daily ticket digest.

use super::{cards, store};
use crate::platform::{caps, db::Db};

const LAST_DIGEST_TICKET: &str = "last_digest_ticket_id";

/// US-30: one email per project to its owner, listing tickets created after the last digest
/// that are still in Triage. No email when there are none, or when the owner's digest card is
/// off. Returns emails queued.
pub async fn digest(db: &Db, base_url: &str, now: i64) -> anyhow::Result<usize> {
    let after = crate::platform::settings::get_raw(&db.read().await?, LAST_DIGEST_TICKET)
        .await?
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(0);
    db.write(async |c| {
        let mut sent = 0;
        let mut last_id = 0;
        loop {
            let batch = crate::projects::after_id(c, last_id, caps::JOB_BATCH).await?;
            last_id = batch.last().map_or(last_id, |p| p.id);
            // A disabled owner has left; ticket titles must not reach their mailbox.
            for p in batch.iter().filter(|p| p.owner_active) {
                sent += queue_one(c, p, after, base_url, now).await?;
            }
            if (batch.len() as i64) < caps::JOB_BATCH {
                break;
            }
        }
        let watermark = crate::tickets::max_id(c).await?;
        crate::platform::settings::set_raw(c, LAST_DIGEST_TICKET, &watermark.to_string()).await?;
        Ok(sent)
    })
    .await
}

/// Queues one project's digest email; returns 1 when it had new tickets, else 0.
async fn queue_one(
    c: &turso::Connection,
    p: &crate::projects::Project,
    after: i64,
    base_url: &str,
    now: i64,
) -> anyhow::Result<usize> {
    if !cards::digest_on(c, p.id).await? {
        return Ok(0);
    }
    let (fresh, total) = crate::tickets::new_in_triage_after(c, p.id, after).await?;
    if fresh.is_empty() {
        return Ok(0);
    }
    let mut lines: Vec<String> = fresh
        .iter()
        .map(|(n, t)| format!("#{n}  {t}\n     {base_url}/projects/{}/tickets/{n}", p.slug))
        .collect();
    if total > fresh.len() as i64 {
        lines.push(format!(
            "…and {} more on the board.",
            total - fresh.len() as i64
        ));
    }
    let subject = match total {
        1 => format!("[Monita] {}: 1 new ticket", p.name),
        n => format!("[Monita] {}: {n} new tickets", p.name),
    };
    let body = format!(
        "New tickets in Triage for {}:\n\n{}\n\nOpen the board: {base_url}/projects/{}/tickets\n",
        p.name,
        lines.join("\n"),
        p.slug
    );
    store::queue(c, "email", &p.owner_email, Some(&subject), &body, now).await?;
    Ok(1)
}
