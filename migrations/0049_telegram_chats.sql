-- Chats a project proved with a connect link (security audit 2026-10-27). A telegram_bot card may
-- only name one of them, so a tech lead cannot point the shared bot at any chat it is in. The
-- chats that existing telegram_bot cards already use count as proved, so no alert stops.
CREATE TABLE telegram_chats (
    project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    chat_id TEXT NOT NULL,
    title TEXT NOT NULL DEFAULT '',
    proved_at INTEGER NOT NULL DEFAULT (unixepoch()),
    PRIMARY KEY (project_id, chat_id)
);

INSERT OR IGNORE INTO telegram_chats (project_id, chat_id, title)
SELECT project_id, "key", name FROM alert_cards
WHERE kind = 'telegram_bot' AND "key" IS NOT NULL AND "key" != '';
