-- Monita v1 schema. Spec: docs/technical-specs/06-data-model.md. Append-only: never edit after merge.

CREATE TABLE users (
  id INTEGER PRIMARY KEY,
  email TEXT NOT NULL UNIQUE,
  password_hash TEXT,
  role TEXT NOT NULL CHECK (role IN ('super_admin', 'tl')),
  disabled_at INTEGER,
  created_at INTEGER NOT NULL DEFAULT (unixepoch()),
  updated_at INTEGER NOT NULL DEFAULT (unixepoch())
);

CREATE TABLE invites (
  id INTEGER PRIMARY KEY,
  user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  token_hash TEXT NOT NULL UNIQUE,
  expires_at INTEGER NOT NULL,
  used_at INTEGER,
  created_at INTEGER NOT NULL DEFAULT (unixepoch())
);

CREATE TABLE sessions (
  id INTEGER PRIMARY KEY,
  user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  token_hash TEXT NOT NULL UNIQUE,
  expires_at INTEGER NOT NULL,
  created_at INTEGER NOT NULL DEFAULT (unixepoch())
);

CREATE TABLE projects (
  id INTEGER PRIMARY KEY,
  owner_id INTEGER NOT NULL REFERENCES users(id),
  name TEXT NOT NULL,
  slug TEXT NOT NULL UNIQUE,
  ip_allowlist TEXT,
  created_at INTEGER NOT NULL DEFAULT (unixepoch()),
  updated_at INTEGER NOT NULL DEFAULT (unixepoch())
);

CREATE TABLE api_keys (
  id INTEGER PRIMARY KEY,
  project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  key_hash TEXT NOT NULL UNIQUE,
  key_prefix TEXT NOT NULL,
  last_used_at INTEGER,
  revoked_at INTEGER,
  created_at INTEGER NOT NULL DEFAULT (unixepoch())
);

CREATE TABLE alert_routes (
  id INTEGER PRIMARY KEY,
  project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  severity TEXT NOT NULL CHECK (severity IN ('critical', 'warning')),
  emails TEXT NOT NULL DEFAULT '',
  webhook_url TEXT,
  use_webhook INTEGER NOT NULL CHECK (use_webhook IN (0, 1)),
  UNIQUE (project_id, severity)
);

CREATE TABLE monitors (
  id INTEGER PRIMARY KEY,
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
CREATE INDEX monitors_next_check ON monitors (next_check_at);

CREATE TABLE heartbeats (
  id INTEGER PRIMARY KEY,
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
CREATE INDEX heartbeats_deadline ON heartbeats (deadline_at);

CREATE TABLE check_results (
  id INTEGER PRIMARY KEY,
  monitor_id INTEGER NOT NULL REFERENCES monitors(id) ON DELETE CASCADE,
  checked_at INTEGER NOT NULL,
  ok INTEGER NOT NULL CHECK (ok IN (0, 1)),
  status_code INTEGER,
  latency_ms INTEGER,
  error TEXT
);
CREATE INDEX check_results_monitor_at ON check_results (monitor_id, checked_at);

CREATE TABLE pings (
  id INTEGER PRIMARY KEY,
  heartbeat_id INTEGER NOT NULL REFERENCES heartbeats(id) ON DELETE CASCADE,
  received_at INTEGER NOT NULL,
  kind TEXT NOT NULL CHECK (kind IN ('ok', 'fail')),
  source_ip TEXT
);
CREATE INDEX pings_heartbeat_at ON pings (heartbeat_id, received_at);

CREATE TABLE daily_summaries (
  source_kind TEXT NOT NULL CHECK (source_kind IN ('monitor', 'heartbeat')),
  source_id INTEGER NOT NULL,
  day TEXT NOT NULL,
  checks_total INTEGER NOT NULL DEFAULT 0,
  checks_ok INTEGER NOT NULL DEFAULT 0,
  downtime_s INTEGER NOT NULL DEFAULT 0,
  avg_latency_ms INTEGER,
  PRIMARY KEY (source_kind, source_id, day)
);

CREATE TABLE incidents (
  id INTEGER PRIMARY KEY,
  project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  origin TEXT NOT NULL CHECK (origin IN ('auto', 'manual')),
  title TEXT NOT NULL,
  severity TEXT NOT NULL CHECK (severity IN ('critical', 'warning')),
  cause TEXT,
  started_at INTEGER NOT NULL,
  resolved_at INTEGER,
  created_by INTEGER REFERENCES users(id) ON DELETE SET NULL
);
CREATE INDEX incidents_project_started ON incidents (project_id, started_at);

CREATE TABLE incident_items (
  incident_id INTEGER NOT NULL REFERENCES incidents(id) ON DELETE CASCADE,
  source_kind TEXT NOT NULL CHECK (source_kind IN ('monitor', 'heartbeat')),
  source_id INTEGER NOT NULL,
  PRIMARY KEY (incident_id, source_kind, source_id)
);
CREATE INDEX incident_items_source ON incident_items (source_kind, source_id);

CREATE TABLE incident_sources (
  source_kind TEXT NOT NULL CHECK (source_kind IN ('monitor', 'heartbeat')),
  source_id INTEGER NOT NULL,
  fail_streak INTEGER NOT NULL DEFAULT 0,
  open_incident_id INTEGER REFERENCES incidents(id) ON DELETE SET NULL,
  PRIMARY KEY (source_kind, source_id)
);

CREATE TABLE status_updates (
  id INTEGER PRIMARY KEY,
  incident_id INTEGER NOT NULL REFERENCES incidents(id) ON DELETE CASCADE,
  label TEXT NOT NULL CHECK (label IN ('investigating', 'identified', 'monitoring', 'resolved')),
  message TEXT NOT NULL,
  author_id INTEGER REFERENCES users(id) ON DELETE SET NULL,
  created_at INTEGER NOT NULL DEFAULT (unixepoch())
);
CREATE INDEX status_updates_incident ON status_updates (incident_id, created_at);

CREATE TABLE outbox (
  id INTEGER PRIMARY KEY,
  channel TEXT NOT NULL CHECK (channel IN ('email', 'webhook')),
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
CREATE INDEX outbox_due ON outbox (sent_at, failed_at, next_attempt_at);

CREATE TABLE ref_ids (
  id INTEGER PRIMARY KEY,
  project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  token_hash TEXT NOT NULL UNIQUE,
  single_use INTEGER NOT NULL DEFAULT 0 CHECK (single_use IN (0, 1)),
  used_at INTEGER,
  revoked_at INTEGER,
  created_at INTEGER NOT NULL DEFAULT (unixepoch())
);

CREATE TABLE tickets (
  id INTEGER PRIMARY KEY,
  project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  number INTEGER NOT NULL,
  ref_id INTEGER NOT NULL REFERENCES ref_ids(id) ON DELETE CASCADE,
  title TEXT NOT NULL,
  public_title TEXT,
  description TEXT NOT NULL,
  page_url TEXT,
  stage TEXT NOT NULL DEFAULT 'triage' CHECK (stage IN ('triage', 'backlog', 'in_progress', 'done')),
  close_reason TEXT CHECK (close_reason IS NULL OR close_reason IN ('fixed', 'wont_fix', 'duplicate')),
  created_at INTEGER NOT NULL DEFAULT (unixepoch()),
  updated_at INTEGER NOT NULL DEFAULT (unixepoch()),
  UNIQUE (project_id, number),
  CHECK ((stage = 'done') = (close_reason IS NOT NULL))
);
CREATE INDEX tickets_project_stage ON tickets (project_id, stage);
CREATE INDEX tickets_ref_created ON tickets (ref_id, created_at);

CREATE TABLE settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
INSERT INTO settings (key, value) VALUES
  ('check_concurrency', '50'),
  ('rl_ip_per_min', '60'),
  ('rl_key_per_min', '120'),
  ('rl_ping_per_s', '1'),
  ('tickets_per_ref_per_day', '5'),
  ('tickets_per_project_per_day', '100');
