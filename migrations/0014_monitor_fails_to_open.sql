-- Each monitor chooses how many failed checks in a row count as down: 1, 2, 3 or 5. It was 2 for
-- every monitor, so 2 stays the default.
ALTER TABLE monitors ADD COLUMN fails_to_open INTEGER NOT NULL DEFAULT 2 CHECK (fails_to_open IN (1, 2, 3, 5));
