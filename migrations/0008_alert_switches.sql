-- An on/off switch per alert setting. Off keeps the recipients and the webhook, so turning it
-- back on restores them. Everything starts on, as it behaved before.
ALTER TABLE alert_routes ADD COLUMN enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1));
ALTER TABLE ticket_alerts ADD COLUMN enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1));
