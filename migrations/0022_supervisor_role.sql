-- A third role, supervisor: sees what a super admin sees and changes nothing. SQLite cannot
-- change a column's CHECK, so this rebuilds users with foreign keys off; every row keeps its id,
-- so sessions, invites, projects and tickets still point at the same people.
CREATE TABLE users_new (
  id INTEGER PRIMARY KEY,
  email TEXT NOT NULL UNIQUE,
  password_hash TEXT,
  role TEXT NOT NULL CHECK (role IN ('super_admin', 'tl', 'supervisor')),
  disabled_at INTEGER,
  created_at INTEGER NOT NULL DEFAULT (unixepoch()),
  updated_at INTEGER NOT NULL DEFAULT (unixepoch())
);
INSERT INTO users_new (id, email, password_hash, role, disabled_at, created_at, updated_at)
  SELECT id, email, password_hash, role, disabled_at, created_at, updated_at FROM users;
DROP TABLE users;
ALTER TABLE users_new RENAME TO users;
