-- Monitor and heartbeat ids are never reused. incident_items, incident_sources and
-- daily_summaries name a source by (kind, id) without a foreign key, so a reused id would
-- inherit a deleted source's incidents and downtime. SQLite's table-rebuild procedure; the
-- runner turns foreign keys off around it, and the copy keeps every id, so references stay valid.
-- The placeholder row (project 0) only moves the AUTOINCREMENT counter; it is deleted at once.
CREATE TABLE monitors_new (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  url TEXT NOT NULL,
  interval_s INTEGER NOT NULL DEFAULT 60 CHECK (interval_s IN (30, 60, 180, 300)),
  timeout_ms INTEGER NOT NULL DEFAULT 10000 CHECK (timeout_ms BETWEEN 1000 AND 30000),
  expected_status TEXT NOT NULL DEFAULT '2xx',
  keyword TEXT,
  skip_tls_verify INTEGER NOT NULL DEFAULT 0 CHECK (skip_tls_verify IN (0, 1)),
  severity TEXT NOT NULL DEFAULT 'warning' CHECK (severity IN ('critical', 'warning')),
  is_public INTEGER NOT NULL DEFAULT 0 CHECK (is_public IN (0, 1)),
  next_check_at INTEGER NOT NULL,
  created_at INTEGER NOT NULL DEFAULT (unixepoch()),
  updated_at INTEGER NOT NULL DEFAULT (unixepoch())
);
-- Start the counter above any id a reference still names, then copy every row.
INSERT INTO monitors_new (id, project_id, name, url, next_check_at)
  SELECT m, 0, '', '', 0 FROM (
    SELECT max(source_id) AS m FROM (
      SELECT source_id FROM incident_items WHERE source_kind = 'monitor'
      UNION ALL SELECT source_id FROM incident_sources WHERE source_kind = 'monitor'
      UNION ALL SELECT source_id FROM daily_summaries WHERE source_kind = 'monitor'))
  WHERE m > (SELECT coalesce(max(id), 0) FROM monitors);
DELETE FROM monitors_new WHERE project_id = 0;
INSERT INTO monitors_new SELECT * FROM monitors;
DROP TABLE monitors;
ALTER TABLE monitors_new RENAME TO monitors;
CREATE INDEX monitors_next_check ON monitors (next_check_at);

CREATE TABLE heartbeats_new (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  token TEXT NOT NULL UNIQUE,
  period_s INTEGER NOT NULL CHECK (period_s BETWEEN 60 AND 2592000),
  grace_s INTEGER NOT NULL CHECK (grace_s BETWEEN 0 AND 86400),
  severity TEXT NOT NULL DEFAULT 'warning' CHECK (severity IN ('critical', 'warning')),
  is_public INTEGER NOT NULL DEFAULT 0 CHECK (is_public IN (0, 1)),
  last_ping_at INTEGER,
  deadline_at INTEGER,
  created_at INTEGER NOT NULL DEFAULT (unixepoch()),
  updated_at INTEGER NOT NULL DEFAULT (unixepoch())
);
-- Start the counter above any id a reference still names, then copy every row.
INSERT INTO heartbeats_new (id, project_id, name, token, period_s, grace_s)
  SELECT m, 0, '', 'placeholder', 60, 0 FROM (
    SELECT max(source_id) AS m FROM (
      SELECT source_id FROM incident_items WHERE source_kind = 'heartbeat'
      UNION ALL SELECT source_id FROM incident_sources WHERE source_kind = 'heartbeat'
      UNION ALL SELECT source_id FROM daily_summaries WHERE source_kind = 'heartbeat'))
  WHERE m > (SELECT coalesce(max(id), 0) FROM heartbeats);
DELETE FROM heartbeats_new WHERE project_id = 0;
INSERT INTO heartbeats_new SELECT * FROM heartbeats;
DROP TABLE heartbeats;
ALTER TABLE heartbeats_new RENAME TO heartbeats;
CREATE INDEX heartbeats_deadline ON heartbeats (deadline_at);
