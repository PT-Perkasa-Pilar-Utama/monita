-- Checks from more than one place (v1.1 decision Q26, technical-specs 14). A location is another
-- machine that runs `monita probe`; only its token's SHA-256 is stored.
CREATE TABLE locations (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL UNIQUE,
  token_hash TEXT NOT NULL UNIQUE,
  token_prefix TEXT NOT NULL,
  last_seen_at INTEGER,
  created_at INTEGER NOT NULL
);

-- The locations a monitor also checks from, each with its own schedule.
CREATE TABLE monitor_locations (
  monitor_id INTEGER NOT NULL REFERENCES monitors(id) ON DELETE CASCADE,
  location_id INTEGER NOT NULL REFERENCES locations(id) ON DELETE CASCADE,
  next_check_at INTEGER NOT NULL,
  PRIMARY KEY (monitor_id, location_id)
);
CREATE INDEX monitor_locations_due ON monitor_locations (location_id, next_check_at);

-- How many places (this server and its locations) must fail for a check to count as failed.
ALTER TABLE monitors ADD COLUMN quorum INTEGER NOT NULL DEFAULT 1 CHECK (quorum BETWEEN 1 AND 11);

-- Null: this server. Deleting a location deletes its results, so none turn into this server's.
ALTER TABLE check_results ADD COLUMN location_id INTEGER REFERENCES locations(id) ON DELETE CASCADE;
CREATE INDEX check_results_place ON check_results (monitor_id, location_id, checked_at);
