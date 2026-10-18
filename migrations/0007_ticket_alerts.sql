-- Ticket alerts, apart from incident severities: the owner's daily digest of new Triage tickets
-- (on by default) and a webhook for each new ticket with its own URL (off by default). Before
-- this, every new ticket called the warning webhook; a project with that webhook on keeps it.
CREATE TABLE ticket_alerts (
  project_id INTEGER PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,
  digest INTEGER NOT NULL DEFAULT 1 CHECK (digest IN (0, 1)),
  webhook_url TEXT,
  use_webhook INTEGER NOT NULL DEFAULT 0 CHECK (use_webhook IN (0, 1)),
  CHECK (use_webhook = 0 OR webhook_url IS NOT NULL)
);
INSERT INTO ticket_alerts (project_id, webhook_url, use_webhook)
  SELECT p.id, r.webhook_url, coalesce(r.use_webhook = 1 AND r.webhook_url IS NOT NULL, 0)
  FROM projects p LEFT JOIN alert_routes r ON r.project_id = p.id AND r.severity = 'warning';
