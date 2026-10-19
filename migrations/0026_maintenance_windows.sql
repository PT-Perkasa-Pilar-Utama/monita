-- Planned work on a project (v1.1 decision Q18). While now is in [starts_at, ends_at), a failure
-- of a covered monitor or heartbeat opens no incident and sends no alert. The state (scheduled,
-- active, ended) follows from the times and is not stored. Monitor and heartbeat ids are
-- AUTOINCREMENT, so an item row left by a deleted source never covers a new one. End now in a
-- window's first second makes ends_at equal starts_at: the window has ended.
CREATE TABLE maintenance_windows (
  id INTEGER PRIMARY KEY,
  project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  title TEXT NOT NULL,
  message TEXT NOT NULL DEFAULT '',
  starts_at INTEGER NOT NULL,
  ends_at INTEGER NOT NULL,
  created_at INTEGER NOT NULL,
  CHECK (ends_at >= starts_at)
);
CREATE INDEX maintenance_windows_project ON maintenance_windows (project_id, ends_at);

CREATE TABLE maintenance_items (
  window_id INTEGER NOT NULL REFERENCES maintenance_windows(id) ON DELETE CASCADE,
  source_kind TEXT NOT NULL CHECK (source_kind IN ('monitor', 'heartbeat')),
  source_id INTEGER NOT NULL,
  PRIMARY KEY (window_id, source_kind, source_id)
);
CREATE INDEX maintenance_items_source ON maintenance_items (source_kind, source_id);
