-- v1.2 Q63: one post-mortem per resolved incident, written by the tech lead, and the addresses
-- that get it when it is published. Both belong to `reports`; no public page reads them.
CREATE TABLE postmortems (
    incident_id INTEGER PRIMARY KEY REFERENCES incidents(id) ON DELETE CASCADE,
    project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    state TEXT NOT NULL CHECK (state IN ('draft', 'published')),
    summary TEXT NOT NULL DEFAULT '',
    impact TEXT NOT NULL DEFAULT '',
    root_cause TEXT NOT NULL DEFAULT '',
    trigger_cause TEXT NOT NULL DEFAULT '',
    resolution TEXT NOT NULL DEFAULT '',
    detection TEXT NOT NULL DEFAULT '',
    action_items TEXT NOT NULL DEFAULT '',
    lessons TEXT NOT NULL DEFAULT '',
    updated_by TEXT NOT NULL,
    updated_at INTEGER NOT NULL,
    -- The first time it was published; a later publish is an update.
    published_at INTEGER
);

CREATE TABLE postmortem_recipients (
    project_id INTEGER PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,
    enabled INTEGER NOT NULL DEFAULT 0 CHECK (enabled IN (0, 1)),
    emails TEXT NOT NULL DEFAULT ''
);
