-- Each monitor and heartbeat chooses whether its failures alert and whether they open an incident;
-- both start on, as before. A source can now be down with no incident, so incident_sources keeps
-- when it went down; an open automatic incident's start time fills it for sources down today.
ALTER TABLE monitors ADD COLUMN alert INTEGER NOT NULL DEFAULT 1 CHECK (alert IN (0, 1));
ALTER TABLE monitors ADD COLUMN auto_incident INTEGER NOT NULL DEFAULT 1 CHECK (auto_incident IN (0, 1));
ALTER TABLE heartbeats ADD COLUMN alert INTEGER NOT NULL DEFAULT 1 CHECK (alert IN (0, 1));
ALTER TABLE heartbeats ADD COLUMN auto_incident INTEGER NOT NULL DEFAULT 1 CHECK (auto_incident IN (0, 1));
ALTER TABLE incident_sources ADD COLUMN down_since INTEGER;
UPDATE incident_sources SET down_since = (SELECT started_at FROM incidents WHERE incidents.id = incident_sources.open_incident_id)
  WHERE open_incident_id IS NOT NULL;
