-- Alert cards (v1.1 Q34) replace the two alert routes. A project holds two fixed cards that email
-- its owner, one for critical and one for warning, and any number of its own up to the cap: an
-- email list or a webhook in one format, each listening to critical, warning or both, each with
-- its own switch. The owner's address is looked up when an alert goes out, so it follows a new
-- owner.
CREATE TABLE alert_cards (
  id INTEGER PRIMARY KEY,
  project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  kind TEXT NOT NULL
    CHECK (kind IN ('owner_email', 'email', 'json', 'discord', 'telegram', 'telegram_bot', 'ntfy', 'pagerduty')),
  name TEXT NOT NULL,
  critical INTEGER NOT NULL CHECK (critical IN (0, 1)),
  warning INTEGER NOT NULL CHECK (warning IN (0, 1)),
  enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
  emails TEXT NOT NULL DEFAULT '',
  url TEXT,
  key TEXT,
  created_at INTEGER NOT NULL DEFAULT (unixepoch()),
  CHECK (critical = 1 OR warning = 1)
);
CREATE INDEX alert_cards_project ON alert_cards (project_id);

-- The owner's cards. On when the route was on and listed the owner; a project without a route
-- gets them on. Addresses are matched whole, between line breaks, so a@x.co never matches aa@x.co.
INSERT INTO alert_cards (project_id, kind, name, critical, warning, enabled)
SELECT p.id, 'owner_email', 'Owner email', s.critical, s.warning,
  CASE
    WHEN r.id IS NULL THEN 1
    WHEN r.enabled = 1 AND instr(
      char(10) || replace(replace(r.emails, ',', char(10)), ' ', '') || char(10),
      char(10) || u.email || char(10)) > 0 THEN 1
    ELSE 0
  END
FROM projects p
JOIN users u ON u.id = p.owner_id
CROSS JOIN (SELECT 'critical' AS sev, 1 AS critical, 0 AS warning
            UNION ALL SELECT 'warning', 0, 1) s
LEFT JOIN alert_routes r ON r.project_id = p.id AND r.severity = s.sev;

-- A route that emailed anyone but the owner alone keeps its whole list in an Email card. The
-- owner may then be in two cards; an alert emails each address once.
INSERT INTO alert_cards (project_id, kind, name, critical, warning, enabled, emails)
SELECT r.project_id, 'email', 'Email', r.severity = 'critical', r.severity = 'warning', r.enabled,
  trim(replace(replace(r.emails, ',', char(10)), ' ', ''), char(10))
FROM alert_routes r
JOIN projects p ON p.id = r.project_id
JOIN users u ON u.id = p.owner_id
WHERE trim(replace(replace(r.emails, ',', char(10)), ' ', ''), char(10)) NOT IN ('', u.email);

-- Each webhook becomes a card in its format. A URL kept with the box unticked becomes a card
-- that is off, so nothing typed is lost.
INSERT INTO alert_cards (project_id, kind, name, critical, warning, enabled, url, key)
SELECT r.project_id, r.webhook_format,
  CASE r.webhook_format
    WHEN 'json' THEN 'Webhook' WHEN 'discord' THEN 'Discord' WHEN 'telegram' THEN 'Telegram'
    WHEN 'telegram_bot' THEN 'Telegram' WHEN 'ntfy' THEN 'ntfy' ELSE 'PagerDuty'
  END,
  r.severity = 'critical', r.severity = 'warning', r.enabled = 1 AND r.use_webhook = 1,
  r.webhook_url, r.webhook_key
FROM alert_routes r
WHERE r.webhook_url IS NOT NULL AND trim(r.webhook_url) <> '';

DROP TABLE alert_routes;

-- A Telegram connect link now fills a card being edited, which may not exist yet, so it belongs
-- to the project. Links last 10 minutes; any open one is dropped.
DROP TABLE telegram_links;
CREATE TABLE telegram_links (
  code_hash TEXT PRIMARY KEY,
  project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  expires_at INTEGER NOT NULL,
  chat_id TEXT,
  chat_title TEXT
);
CREATE INDEX telegram_links_project ON telegram_links (project_id);
