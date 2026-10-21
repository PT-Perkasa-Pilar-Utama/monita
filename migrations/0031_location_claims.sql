-- A location's result must answer a check it claimed (technical-specs 14.6): the claim's time
-- until the result arrives, then NULL. One result per claim, so a probe cannot store more rows
-- than it was handed checks.
ALTER TABLE monitor_locations ADD COLUMN claimed_at INTEGER;
