-- Sealed secrets per project: written once, never shown again, replaced or deleted. A monitor's
-- authentication reads one by id. The value is stored as entered: the check must send it.
CREATE TABLE project_secrets (
  id INTEGER PRIMARY KEY,
  project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  value TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE (project_id, name)
);
ALTER TABLE monitors ADD COLUMN auth_kind TEXT NOT NULL DEFAULT 'none' CHECK (auth_kind IN ('none', 'bearer', 'basic', 'header'));
ALTER TABLE monitors ADD COLUMN auth_secret_id INTEGER REFERENCES project_secrets(id) ON DELETE SET NULL;
ALTER TABLE monitors ADD COLUMN auth_user TEXT;
ALTER TABLE monitors ADD COLUMN auth_header TEXT;
