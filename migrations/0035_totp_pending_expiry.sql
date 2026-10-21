-- A waiting two-factor key lasts 10 minutes (v1.1 Q32). Keys that waited before this had no end,
-- so they are dropped; those users select Turn on again.
ALTER TABLE users ADD COLUMN totp_pending_expires_at INTEGER;
UPDATE users SET totp_pending = NULL WHERE totp_pending IS NOT NULL;
