-- A team member files a ticket without a ref ID: ref_id turns nullable and created_by names
-- the member. A reporter's ticket keeps its ref ID. SQLite's table-rebuild procedure; the
-- runner turns foreign keys off around it, and the copy keeps every id.
CREATE TABLE tickets_new (
  id INTEGER PRIMARY KEY,
  project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  number INTEGER NOT NULL,
  ref_id INTEGER REFERENCES ref_ids(id) ON DELETE CASCADE,
  title TEXT NOT NULL,
  public_title TEXT,
  description TEXT NOT NULL,
  page_url TEXT,
  stage TEXT NOT NULL DEFAULT 'triage' CHECK (stage IN ('triage', 'backlog', 'in_progress', 'done')),
  close_reason TEXT CHECK (close_reason IS NULL OR close_reason IN ('fixed', 'wont_fix', 'duplicate')),
  created_at INTEGER NOT NULL DEFAULT (unixepoch()),
  updated_at INTEGER NOT NULL DEFAULT (unixepoch()),
  created_by INTEGER REFERENCES users(id) ON DELETE SET NULL,
  UNIQUE (project_id, number),
  CHECK ((stage = 'done') = (close_reason IS NOT NULL)),
  CHECK (ref_id IS NULL OR created_by IS NULL)
);
INSERT INTO tickets_new (id, project_id, number, ref_id, title, public_title, description, page_url, stage, close_reason, created_at, updated_at)
  SELECT id, project_id, number, ref_id, title, public_title, description, page_url, stage, close_reason, created_at, updated_at FROM tickets;
DROP TABLE tickets;
ALTER TABLE tickets_new RENAME TO tickets;
CREATE INDEX tickets_project_stage ON tickets (project_id, stage);
CREATE INDEX tickets_ref_created ON tickets (ref_id, created_at);
