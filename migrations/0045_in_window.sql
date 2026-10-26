-- A check result or ping that arrived inside an active maintenance window (v1.1 Q46). Charts,
-- sparklines and the daily rollup count such a failure as held, not as a failure, so planned
-- work leaves no noise. Rows from before this column read 0.
ALTER TABLE check_results ADD COLUMN in_window INTEGER NOT NULL DEFAULT 0;
ALTER TABLE pings ADD COLUMN in_window INTEGER NOT NULL DEFAULT 0;
