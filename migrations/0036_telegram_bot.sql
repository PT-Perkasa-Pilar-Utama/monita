-- Monita's own Telegram bot (v1.1 Q33).
--
-- Connect links: a project owner opens one; the bot hears `/start <code>` in a chat, and the
-- chat's ID fills the severity's form. Only the code's SHA-256 is kept. A link lasts 10 minutes.
CREATE TABLE telegram_links (
  code_hash TEXT PRIMARY KEY,
  project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  severity TEXT NOT NULL CHECK (severity IN ('critical', 'warning')),
  expires_at INTEGER NOT NULL,
  chat_id TEXT,
  chat_title TEXT
);
CREATE INDEX telegram_links_route ON telegram_links (project_id, severity);

-- A severity's webhook may use the bot (`telegram_bot`), and the outbox gets a `telegram` channel,
-- whose rows name a chat; delivery adds the token. SQLite cannot change a CHECK, so both tables
-- are rebuilt with foreign keys off; every row keeps its id.
CREATE TABLE alert_routes_new (
  id INTEGER PRIMARY KEY,
  project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  severity TEXT NOT NULL CHECK (severity IN ('critical', 'warning')),
  emails TEXT NOT NULL DEFAULT '',
  webhook_url TEXT,
  use_webhook INTEGER NOT NULL CHECK (use_webhook IN (0, 1)),
  enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
  webhook_format TEXT NOT NULL DEFAULT 'json'
    CHECK (webhook_format IN ('json', 'discord', 'telegram', 'telegram_bot', 'ntfy', 'pagerduty')),
  webhook_key TEXT,
  UNIQUE (project_id, severity)
);
INSERT INTO alert_routes_new (id, project_id, severity, emails, webhook_url, use_webhook, enabled,
    webhook_format, webhook_key)
  SELECT id, project_id, severity, emails, webhook_url, use_webhook, enabled, webhook_format, webhook_key
  FROM alert_routes;
DROP TABLE alert_routes;
ALTER TABLE alert_routes_new RENAME TO alert_routes;

CREATE TABLE outbox_new (
  id INTEGER PRIMARY KEY,
  channel TEXT NOT NULL CHECK (channel IN ('email', 'webhook', 'telegram')),
  recipient TEXT NOT NULL,
  subject TEXT,
  body TEXT NOT NULL,
  attempts INTEGER NOT NULL DEFAULT 0,
  next_attempt_at INTEGER NOT NULL,
  sent_at INTEGER,
  failed_at INTEGER,
  last_error TEXT,
  created_at INTEGER NOT NULL DEFAULT (unixepoch())
);
INSERT INTO outbox_new (id, channel, recipient, subject, body, attempts, next_attempt_at, sent_at,
    failed_at, last_error, created_at)
  SELECT id, channel, recipient, subject, body, attempts, next_attempt_at, sent_at, failed_at,
    last_error, created_at
  FROM outbox;
DROP TABLE outbox;
ALTER TABLE outbox_new RENAME TO outbox;
CREATE INDEX outbox_due ON outbox (sent_at, failed_at, next_attempt_at);
