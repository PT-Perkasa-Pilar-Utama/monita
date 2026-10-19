-- A project's releases, shown on the public Changelog tab one at a time, newest first.
-- released_on is a YYYY-MM-DD day, as typed. sections is JSON: [{"tag": "Added", "items": [..]}].
CREATE TABLE changelog_entries (
  id INTEGER PRIMARY KEY,
  project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  version TEXT NOT NULL,
  released_on TEXT NOT NULL,
  title TEXT NOT NULL,
  sections TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE (project_id, version)
);
CREATE INDEX changelog_entries_newest ON changelog_entries (project_id, released_on DESC, id DESC);
-- The tab starts off, so no existing status page grows an empty Changelog.
ALTER TABLE projects ADD COLUMN show_changelog INTEGER NOT NULL DEFAULT 0 CHECK (show_changelog IN (0, 1));
