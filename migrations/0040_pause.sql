-- A tech lead pauses a monitor or heartbeat instead of deleting it (v1.1 Q38). While paused_at is
-- set, nothing checks it, no deadline is watched, no incident opens and no alert goes out.
ALTER TABLE monitors ADD COLUMN paused_at INTEGER;
ALTER TABLE heartbeats ADD COLUMN paused_at INTEGER;
