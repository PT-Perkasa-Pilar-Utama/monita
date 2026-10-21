-- A monitor may also ping a host (v1.1 decision Q24). SQLite cannot change a column's CHECK, so
-- this rebuilds monitors as 0018 did: foreign keys are off around it, every row keeps its id, and
-- the placeholder row keeps the AUTOINCREMENT counter above any id a reference still names,
-- maintenance items included.
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
  updated_at INTEGER NOT NULL DEFAULT (unixepoch()),
  alert INTEGER NOT NULL DEFAULT 1 CHECK (alert IN (0, 1)),
  auto_incident INTEGER NOT NULL DEFAULT 1 CHECK (auto_incident IN (0, 1)),
  method TEXT NOT NULL DEFAULT 'GET' CHECK (method IN ('GET', 'HEAD', 'POST', 'PUT')),
  headers TEXT,
  body TEXT,
  fails_to_open INTEGER NOT NULL DEFAULT 2 CHECK (fails_to_open BETWEEN 1 AND 99),
  auth_kind TEXT NOT NULL DEFAULT 'none' CHECK (auth_kind IN ('none', 'bearer', 'basic', 'header')),
  auth_secret_id INTEGER REFERENCES project_secrets(id) ON DELETE SET NULL,
  auth_user TEXT,
  auth_header TEXT,
  kind TEXT NOT NULL DEFAULT 'http' CHECK (kind IN ('http', 'tcp', 'dns', 'ping')),
  expected_address TEXT,
  cert_warn_days INTEGER CHECK (cert_warn_days IN (7, 14, 30)),
  cert_expires_at INTEGER,
  cert_warned_for INTEGER
);
INSERT INTO monitors_new (id, project_id, name, url, next_check_at)
  SELECT m, 0, '', '', 0 FROM (
    SELECT max(source_id) AS m FROM (
      SELECT source_id FROM incident_items WHERE source_kind = 'monitor'
      UNION ALL SELECT source_id FROM incident_sources WHERE source_kind = 'monitor'
      UNION ALL SELECT source_id FROM daily_summaries WHERE source_kind = 'monitor'
      UNION ALL SELECT source_id FROM maintenance_items WHERE source_kind = 'monitor'))
  WHERE m > (SELECT coalesce(max(id), 0) FROM monitors);
DELETE FROM monitors_new WHERE project_id = 0;
INSERT INTO monitors_new (id, project_id, name, url, interval_s, timeout_ms, expected_status, keyword,
    skip_tls_verify, severity, is_public, next_check_at, created_at, updated_at, alert, auto_incident,
    method, headers, body, fails_to_open, auth_kind, auth_secret_id, auth_user, auth_header,
    kind, expected_address, cert_warn_days, cert_expires_at, cert_warned_for)
  SELECT id, project_id, name, url, interval_s, timeout_ms, expected_status, keyword,
    skip_tls_verify, severity, is_public, next_check_at, created_at, updated_at, alert, auto_incident,
    method, headers, body, fails_to_open, auth_kind, auth_secret_id, auth_user, auth_header,
    kind, expected_address, cert_warn_days, cert_expires_at, cert_warned_for
  FROM monitors;
DROP TABLE monitors;
ALTER TABLE monitors_new RENAME TO monitors;
CREATE INDEX monitors_next_check ON monitors (next_check_at);
