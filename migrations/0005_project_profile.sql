-- An optional description and icon per project (api-specs 03). The icon's bytes live in their
-- own table so project lists never read them; icon_at marks that one exists and versions its URL.
ALTER TABLE projects ADD COLUMN description TEXT;
ALTER TABLE projects ADD COLUMN icon_at INTEGER;
CREATE TABLE project_icons (
  project_id INTEGER PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,
  mime TEXT NOT NULL CHECK (mime IN ('image/png', 'image/webp')),
  data BLOB NOT NULL
);
