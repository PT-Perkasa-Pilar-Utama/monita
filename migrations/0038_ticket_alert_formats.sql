-- The ticket webhook speaks a format, as an alert card does (v1.1 Q36): JSON as before, Discord,
-- Telegram with its own bot or the global one, or ntfy. Not PagerDuty: a ticket is no incident,
-- so its trigger would never resolve. webhook_key holds a Telegram chat ID.
ALTER TABLE ticket_alerts ADD COLUMN webhook_format TEXT NOT NULL DEFAULT 'json'
  CHECK (webhook_format IN ('json', 'discord', 'telegram', 'telegram_bot', 'ntfy'));
ALTER TABLE ticket_alerts ADD COLUMN webhook_key TEXT;
