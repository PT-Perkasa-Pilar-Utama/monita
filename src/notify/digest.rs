//! US-30: the daily ticket digest.

use super::store;
use crate::platform::db::Db;

const LAST_DIGEST_TICKET: &str = "last_digest_ticket_id";

/// US-30: one email per project to its owner, listing tickets created after the last digest
/// that are still in Triage. No email when there are none. Returns emails queued.
pub async fn digest(db: &Db, base_url: &str, now: i64) -> anyhow::Result<usize> {
    let after = crate::platform::settings::get_raw(&db.read().await?, LAST_DIGEST_TICKET)
        .await?
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(0);
    let projects = crate::projects::all(db).await?;
    db.write(async |c| {
        let mut sent = 0;
        // A disabled owner has left; ticket titles must not reach their mailbox.
        for p in projects.iter().filter(|p| p.owner_active) {
            let fresh = crate::tickets::new_in_triage_after(c, p.id, after).await?;
            if fresh.is_empty() {
                continue;
            }
            let lines: Vec<String> = fresh.iter().map(|(n, t)| format!("#{n}  {t}\n     {base_url}/projects/{}/tickets/{n}", p.slug)).collect();
            let subject = match fresh.len() {
                1 => format!("[Monita] {}: 1 new ticket", p.name),
                n => format!("[Monita] {}: {n} new tickets", p.name),
            };
            let body = format!("New tickets in Triage for {}:\n\n{}\n\nOpen the board: {base_url}/projects/{}/tickets\n", p.name, lines.join("\n"), p.slug);
            store::queue(c, "email", &p.owner_email, Some(&subject), &body, now).await?;
            sent += 1;
        }
        let watermark = crate::tickets::max_id(c).await?;
        crate::platform::settings::set_raw(c, LAST_DIGEST_TICKET, &watermark.to_string()).await?;
        Ok(sent)
    })
    .await
}
