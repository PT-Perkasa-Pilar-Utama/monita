-- A location's result must answer a check it claimed (technical-specs 14.6). `open_claims` counts
-- the checks handed out and not yet answered, at most 2, so a slow probe that is handed the same
-- check again loses neither result; `claimed_at` is the newest claim, and a result later than two
-- intervals, the timeout and 30 seconds after it is refused as stale.
ALTER TABLE monitor_locations ADD COLUMN claimed_at INTEGER;
ALTER TABLE monitor_locations ADD COLUMN open_claims INTEGER NOT NULL DEFAULT 0;
