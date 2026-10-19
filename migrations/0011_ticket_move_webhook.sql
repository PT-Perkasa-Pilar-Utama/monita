-- The ticket webhook can also hear each column move. Off by default. Turning it on needs the
-- webhook URL; the app checks that, since a table CHECK cannot be added after the fact.
ALTER TABLE ticket_alerts ADD COLUMN webhook_moves INTEGER NOT NULL DEFAULT 0 CHECK (webhook_moves IN (0, 1));
