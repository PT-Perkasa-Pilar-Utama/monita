-- A project API key has a scope (v1.1 Q44): `ref_ids` issues ref IDs only, as every key did
-- before; `read` reads the project's monitors, heartbeats, incidents and maintenance through
-- /api/v1/projects/{slug}/; `write` also changes them and issues ref IDs. Every existing key keeps
-- `ref_ids`, so no deployed key gains power.
ALTER TABLE api_keys ADD COLUMN scope TEXT NOT NULL DEFAULT 'ref_ids'
  CHECK (scope IN ('ref_ids', 'read', 'write'));
