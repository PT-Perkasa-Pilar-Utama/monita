-- Ticket alerts become alert cards (v1.1 Q37). A card hears incidents (critical, warning) or
-- tickets (new tickets, ticket moves, and for the owner's card the daily digest), never both.
-- Every project gets three fixed owner cards for tickets: the daily digest, each new ticket, and
-- each move. ticket_alerts is converted and dropped. SQLite cannot change a CHECK, so alert_cards
-- is rebuilt with foreign keys off; every card keeps its id.
CREATE TABLE alert_cards_new (
  id INTEGER PRIMARY KEY,
  project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  kind TEXT NOT NULL
    CHECK (kind IN ('owner_email', 'email', 'json', 'discord', 'telegram', 'telegram_bot', 'ntfy', 'pagerduty')),
  name TEXT NOT NULL,
  critical INTEGER NOT NULL DEFAULT 0 CHECK (critical IN (0, 1)),
  warning INTEGER NOT NULL DEFAULT 0 CHECK (warning IN (0, 1)),
  new_tickets INTEGER NOT NULL DEFAULT 0 CHECK (new_tickets IN (0, 1)),
  ticket_moves INTEGER NOT NULL DEFAULT 0 CHECK (ticket_moves IN (0, 1)),
  digest INTEGER NOT NULL DEFAULT 0 CHECK (digest IN (0, 1)),
  enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
  emails TEXT NOT NULL DEFAULT '',
  url TEXT,
  key TEXT,
  created_at INTEGER NOT NULL DEFAULT (unixepoch()),
  CHECK (critical + warning + new_tickets + ticket_moves + digest >= 1),
  CHECK (critical + warning = 0 OR new_tickets + ticket_moves + digest = 0)
);
INSERT INTO alert_cards_new (id, project_id, kind, name, critical, warning, enabled, emails, url, key, created_at)
  SELECT id, project_id, kind, name, critical, warning, enabled, emails, url, key, created_at FROM alert_cards;
DROP TABLE alert_cards;
ALTER TABLE alert_cards_new RENAME TO alert_cards;
CREATE INDEX alert_cards_project ON alert_cards (project_id);

-- The owner's ticket cards. The digest keeps what the project had: on unless Ticket alerts or the
-- digest were off. Emails per new ticket and per move are new, so they start off.
INSERT INTO alert_cards (project_id, kind, name, digest, new_tickets, ticket_moves, enabled)
SELECT p.id, 'owner_email', 'Owner email', e.digest, e.new_tickets, e.ticket_moves,
  CASE WHEN e.digest = 1 THEN coalesce(t.enabled = 1 AND t.digest = 1, 1) ELSE 0 END
FROM projects p
CROSS JOIN (SELECT 1 AS digest, 0 AS new_tickets, 0 AS ticket_moves
            UNION ALL SELECT 0, 1, 0 UNION ALL SELECT 0, 0, 1) e
LEFT JOIN ticket_alerts t ON t.project_id = p.id;

-- The ticket webhook becomes a card in its format. A URL kept with both boxes off becomes a card
-- that is off, so nothing typed is lost.
INSERT INTO alert_cards (project_id, kind, name, new_tickets, ticket_moves, enabled, url, key)
SELECT t.project_id, t.webhook_format,
  CASE t.webhook_format
    WHEN 'json' THEN 'JSON webhook' WHEN 'discord' THEN 'Discord' WHEN 'telegram' THEN 'Telegram'
    WHEN 'telegram_bot' THEN 'Global Telegram' ELSE 'ntfy'
  END,
  CASE WHEN t.use_webhook = 0 AND t.webhook_moves = 0 THEN 1 ELSE t.use_webhook END,
  t.webhook_moves,
  t.enabled = 1 AND (t.use_webhook = 1 OR t.webhook_moves = 1),
  t.webhook_url, t.webhook_key
FROM ticket_alerts t
WHERE t.webhook_url IS NOT NULL AND trim(t.webhook_url) <> '';

DROP TABLE ticket_alerts;
