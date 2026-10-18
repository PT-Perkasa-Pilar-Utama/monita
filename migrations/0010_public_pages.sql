-- Each project chooses which public tabs visitors see besides Status, which always shows.
ALTER TABLE projects ADD COLUMN show_known_issues INTEGER NOT NULL DEFAULT 1 CHECK (show_known_issues IN (0, 1));
ALTER TABLE projects ADD COLUMN show_report INTEGER NOT NULL DEFAULT 1 CHECK (show_report IN (0, 1));
ALTER TABLE projects ADD COLUMN show_mine INTEGER NOT NULL DEFAULT 1 CHECK (show_mine IN (0, 1));
