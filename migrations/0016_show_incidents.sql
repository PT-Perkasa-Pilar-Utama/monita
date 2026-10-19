-- The public Incidents tab: open incidents, the last 7 days and the history. Uptime always shows.
ALTER TABLE projects ADD COLUMN show_incidents INTEGER NOT NULL DEFAULT 1 CHECK (show_incidents IN (0, 1));
