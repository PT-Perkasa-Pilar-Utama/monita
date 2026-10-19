-- Each severity's webhook speaks one format (v1.1 decision Q17). `json` is the body Monita sent
-- before. Telegram needs a chat ID and PagerDuty a routing key; both live in webhook_key, which,
-- like the URL, a supervisor never sees.
ALTER TABLE alert_routes ADD COLUMN webhook_format TEXT NOT NULL DEFAULT 'json'
  CHECK (webhook_format IN ('json', 'discord', 'telegram', 'ntfy', 'pagerduty'));
ALTER TABLE alert_routes ADD COLUMN webhook_key TEXT;
