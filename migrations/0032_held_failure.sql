-- A failure a job reports during a maintenance window waits for the window's end (v1.1 Q18).
-- This keeps what the job said, so the incident that opens then carries it. A success ping
-- clears it.
ALTER TABLE heartbeats ADD COLUMN held_cause TEXT;
