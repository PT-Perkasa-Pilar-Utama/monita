-- demo seed. Loaded by POST /admin/reset-state {"seed":"demo"}; the route does not exist in prod.
-- Feeds the static demo (e2e/demo/snapshot.ts). Times are relative to the load, so the snapshot
-- always shows recent history. Monitors are not due for 10 minutes, so nothing is probed.
INSERT INTO users (id, email, role, password_hash, created_at) VALUES
  (1, 'admin@monita.test', 'super_admin', '$argon2id$v=19$m=19456,t=2,p=1$H5MnCFM1Yf3jchQQBWqLsQ$IJOegx6p9PvqbCdN8pb0xnxn0XeWAa73WOozfHbzl9U', unixepoch() - 120 * 86400),
  (2, 'tl@monita.test', 'tl', '$argon2id$v=19$m=19456,t=2,p=1$H5MnCFM1Yf3jchQQBWqLsQ$IJOegx6p9PvqbCdN8pb0xnxn0XeWAa73WOozfHbzl9U', unixepoch() - 110 * 86400),
  (3, 'rina@monita.test', 'tl', '$argon2id$v=19$m=19456,t=2,p=1$H5MnCFM1Yf3jchQQBWqLsQ$IJOegx6p9PvqbCdN8pb0xnxn0XeWAa73WOozfHbzl9U', unixepoch() - 60 * 86400);

INSERT INTO projects (id, owner_id, name, slug, created_at) VALUES
  (1, 2, 'Customer Portal', 'portal', unixepoch() - 100 * 86400),
  (2, 2, 'Payments', 'payments', unixepoch() - 100 * 86400),
  (3, 3, 'Data Warehouse', 'warehouse', unixepoch() - 50 * 86400);

INSERT INTO api_keys (project_id, name, key_hash, key_prefix, last_used_at, created_at) VALUES
  (1, 'portal backend', 'demo-key-hash-1', 'mk_7QX2', unixepoch() - 300, unixepoch() - 90 * 86400),
  (2, 'payments worker', 'demo-key-hash-2', 'mk_K4MP', unixepoch() - 3600, unixepoch() - 80 * 86400);

INSERT INTO alert_routes (project_id, severity, emails, webhook_url, use_webhook) VALUES
  (1, 'critical', 'oncall-portal@example.com', 'https://chat.example.com/hooks/portal', 1),
  (1, 'warning', 'portal-team@example.com', 'https://chat.example.com/hooks/portal', 0),
  (2, 'critical', 'oncall-payments@example.com, finance-it@example.com', 'https://chat.example.com/hooks/payments', 1),
  (2, 'warning', 'payments-team@example.com', NULL, 0),
  (3, 'critical', 'data@example.com', NULL, 0),
  (3, 'warning', 'data@example.com', NULL, 0);

INSERT INTO monitors (id, project_id, name, url, interval_s, expected_status, keyword, severity, is_public, next_check_at, created_at) VALUES
  (1, 1, 'Website', 'https://portal.example.com/', 60, '2xx', NULL, 'critical', 1, unixepoch() + 600, unixepoch() - 100 * 86400),
  (2, 1, 'API', 'https://api.portal.example.com/healthz', 30, '200', NULL, 'critical', 1, unixepoch() + 600, unixepoch() - 100 * 86400),
  (3, 1, 'Login page', 'https://portal.example.com/login', 180, '2xx', 'Sign in', 'warning', 1, unixepoch() + 600, unixepoch() - 95 * 86400),
  (4, 1, 'Admin panel', 'https://admin.portal.example.com/', 300, '2xx,401', NULL, 'warning', 0, unixepoch() + 600, unixepoch() - 70 * 86400),
  (5, 2, 'Checkout API', 'https://pay.example.com/v2/health', 30, '2xx', NULL, 'critical', 1, unixepoch() + 600, unixepoch() - 100 * 86400),
  (6, 2, 'Bank callback', 'https://pay.example.com/callback/ping', 60, '2xx', NULL, 'critical', 1, unixepoch() + 600, unixepoch() - 100 * 86400),
  (7, 3, 'Query gateway', 'https://dwh.example.com/health', 300, '2xx', NULL, 'warning', 0, unixepoch() + 600, unixepoch() - 50 * 86400);

-- The last hour at each monitor's interval; Login page answers slowly in its last ten checks,
-- behind the open manual incident.
INSERT INTO check_results (monitor_id, checked_at, ok, status_code, latency_ms, error)
WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 120)
SELECT m.id, unixepoch() - n.i * m.interval_s, 1, 200,
  CASE WHEN m.id = 3 AND n.i <= 10 THEN 2300 + (n.i * 173) % 900 ELSE 70 + m.id * 12 + (n.i * 37 + m.id * 53) % 60 END,
  NULL
FROM monitors m, n WHERE n.i * m.interval_s <= 3600;

-- Before that, one check every 5 minutes back to 7 days, with a daily wave and a few failures of
-- the bank callback, enough to fill the day and week charts.
INSERT INTO check_results (monitor_id, checked_at, ok, status_code, latency_ms, error)
WITH RECURSIVE n(i) AS (SELECT 13 UNION ALL SELECT i + 1 FROM n WHERE i < 2016)
SELECT m.id, unixepoch() - n.i * 300,
  CASE WHEN m.id = 6 AND n.i IN (40, 41, 300) THEN 0 ELSE 1 END,
  CASE WHEN m.id = 6 AND n.i IN (40, 41, 300) THEN 502 ELSE 200 END,
  70 + m.id * 12 + (n.i * 37 + m.id * 53) % 60 + 45 * ((n.i % 288) BETWEEN 96 AND 180),
  CASE WHEN m.id = 6 AND n.i IN (40, 41, 300) THEN 'HTTP 502 from the bank gateway' END
FROM monitors m, n;

INSERT INTO heartbeats (id, project_id, name, token, period_s, grace_s, severity, is_public, last_ping_at, deadline_at, created_at) VALUES
  (1, 1, 'Nightly backup', 'demo-token-nightly-backup-01', 86400, 1800, 'warning', 1, unixepoch() - 5 * 3600, unixepoch() - 5 * 3600 + 86400 + 1800, unixepoch() - 100 * 86400),
  (2, 1, 'Search index rebuild', 'demo-token-search-index-0002', 3600, 300, 'warning', 0, unixepoch() - 900, unixepoch() - 900 + 3600 + 300, unixepoch() - 60 * 86400),
  (3, 2, 'Settlement export', 'demo-token-settlement-export3', 86400, 1800, 'critical', 1, unixepoch() - 86400 - 1800 - 7200, NULL, unixepoch() - 100 * 86400),
  (4, 3, 'ETL load', 'demo-token-etl-load-000000004', 21600, 900, 'warning', 0, unixepoch() - 4000, unixepoch() - 4000 + 21600 + 900, unixepoch() - 50 * 86400);

INSERT INTO pings (heartbeat_id, received_at, kind, source_ip, cause)
WITH RECURSIVE n(i) AS (SELECT 0 UNION ALL SELECT i + 1 FROM n WHERE i < 13)
SELECT 1, unixepoch() - 5 * 3600 - n.i * 86400, 'ok', '10.20.0.14', NULL FROM n
UNION ALL
SELECT 2, unixepoch() - 900 - n.i * 3600,
  CASE WHEN n.i = 6 THEN 'fail' ELSE 'ok' END, '10.20.0.31',
  CASE WHEN n.i = 6 THEN 'Error: index shard 3 timed out after 120 s' END FROM n
UNION ALL
SELECT 3, unixepoch() - 86400 - 1800 - 7200 - n.i * 86400, 'ok', '10.30.1.5', NULL FROM n;

-- Past automatic incidents give the status page its red days.
INSERT INTO incidents (id, project_id, origin, title, severity, cause, started_at, resolved_at, created_by) VALUES
  (1, 1, 'auto', 'Website is down', 'critical', 'HTTP 502 from https://portal.example.com/', unixepoch() - 40 * 86400, unixepoch() - 40 * 86400 + 480, NULL),
  (2, 1, 'auto', 'API is down', 'critical', 'Connection timed out after 10 s', unixepoch() - 12 * 86400, unixepoch() - 12 * 86400 + 1500, NULL),
  (3, 1, 'auto', 'Login page is down', 'warning', 'Keyword "Sign in" not found', unixepoch() - 63 * 86400, unixepoch() - 63 * 86400 + 10800, NULL),
  (4, 2, 'auto', 'Checkout API is down', 'critical', 'HTTP 503 from https://pay.example.com/v2/health', unixepoch() - 27 * 86400, unixepoch() - 27 * 86400 + 2100, NULL),
  (5, 2, 'manual', 'Card payments declined for one bank', 'critical', NULL, unixepoch() - 20 * 86400, unixepoch() - 20 * 86400 + 5400, 2),
  (6, 2, 'auto', 'Settlement export missed its heartbeat', 'critical', 'No ping within 1 day and 30 minutes', unixepoch() - 7200, NULL, NULL),
  (7, 1, 'manual', 'Slow sign-in for some users', 'warning', NULL, unixepoch() - 3000, NULL, 2),
  (8, 2, 'auto', 'Bank callback is down', 'critical', 'HTTP 502 from the bank gateway', unixepoch() - 3 * 86400, unixepoch() - 3 * 86400 + 360, NULL),
  (9, 1, 'auto', 'Website is down', 'critical', 'Connection reset by peer', unixepoch() - 5 * 86400, unixepoch() - 5 * 86400 + 240, NULL);

INSERT INTO incident_items (incident_id, source_kind, source_id) VALUES
  (1, 'monitor', 1), (2, 'monitor', 2), (3, 'monitor', 3), (4, 'monitor', 5),
  (5, 'monitor', 5), (5, 'monitor', 6), (6, 'heartbeat', 3), (7, 'monitor', 3), (8, 'monitor', 6), (9, 'monitor', 1);

INSERT INTO incident_sources (source_kind, source_id, fail_streak, open_incident_id) VALUES
  ('monitor', 1, 0, NULL), ('monitor', 2, 0, NULL), ('monitor', 3, 0, NULL), ('monitor', 4, 0, NULL),
  ('monitor', 5, 0, NULL), ('monitor', 6, 0, NULL), ('monitor', 7, 0, NULL),
  ('heartbeat', 3, 0, 6);

INSERT INTO status_updates (incident_id, label, message, author_id, created_at) VALUES
  (2, 'resolved', 'A database failover stalled the API. It recovered on its own.', 2, unixepoch() - 12 * 86400 + 1600),
  (5, 'investigating', 'Card payments from one bank are being declined. Other banks work.', 2, unixepoch() - 20 * 86400 + 300),
  (5, 'identified', 'The bank rotated its certificate. We are installing the new one.', 2, unixepoch() - 20 * 86400 + 2400),
  (5, 'resolved', 'The new certificate is installed and payments go through.', 2, unixepoch() - 20 * 86400 + 5400),
  (6, 'investigating', 'The nightly settlement file did not arrive. We are checking the export job.', 2, unixepoch() - 5400),
  (7, 'investigating', 'Some users wait more than 2 s to sign in.', 2, unixepoch() - 2700),
  (7, 'identified', 'A slow session-store node. Traffic is moving to the others.', 2, unixepoch() - 1200),
  (8, 'resolved', 'The bank restarted its gateway. Callbacks are flowing again.', 2, unixepoch() - 3 * 86400 + 900);

INSERT INTO ref_ids (id, project_id, token_hash, single_use, used_at, created_at) VALUES
  (1, 1, 'demo-ref-hash-1', 0, NULL, unixepoch() - 30 * 86400),
  (2, 1, 'demo-ref-hash-2', 0, NULL, unixepoch() - 20 * 86400),
  (3, 1, 'demo-ref-hash-3', 1, unixepoch() - 3 * 86400, unixepoch() - 4 * 86400),
  (4, 2, 'demo-ref-hash-4', 0, NULL, unixepoch() - 15 * 86400);

INSERT INTO tickets (project_id, number, ref_id, title, public_title, description, page_url, stage, close_reason, created_at, updated_at) VALUES
  (1, 1, 1, 'Invoice PDF shows the wrong month', 'Invoice PDF shows the wrong month', 'The PDF for my October invoice says September in the header.', 'https://portal.example.com/billing', 'done', 'fixed', unixepoch() - 28 * 86400, unixepoch() - 21 * 86400),
  (1, 2, 1, 'Cannot upload a profile photo over 2 MB', 'Profile photos over 2 MB are refused', 'The upload spins forever with a 3 MB JPEG.', 'https://portal.example.com/profile', 'in_progress', NULL, unixepoch() - 18 * 86400, unixepoch() - 2 * 86400),
  (1, 3, 2, 'Dark mode makes the table headers unreadable', NULL, 'Grey text on a dark grey header in the orders table.', 'https://portal.example.com/orders', 'backlog', NULL, unixepoch() - 15 * 86400, unixepoch() - 10 * 86400),
  (1, 4, 2, 'Export to CSV drops accented names', 'CSV export loses accented characters', 'José becomes Jos? in the CSV.', 'https://portal.example.com/orders', 'backlog', NULL, unixepoch() - 9 * 86400, unixepoch() - 8 * 86400),
  (1, 5, 3, 'Password reset email never arrives', NULL, 'I tried three times. Nothing in spam either. My address is on a company domain.', 'https://portal.example.com/forgot', 'triage', NULL, unixepoch() - 3 * 86400, unixepoch() - 3 * 86400),
  (1, 6, 2, 'Same as the CSV accents bug', NULL, 'Names with ñ break in the export too.', 'https://portal.example.com/orders', 'done', 'duplicate', unixepoch() - 7 * 86400, unixepoch() - 6 * 86400),
  (1, 7, 1, 'Search ignores the date filter', NULL, 'Filtering orders by last week still shows older ones.', 'https://portal.example.com/orders', 'triage', NULL, unixepoch() - 86400, unixepoch() - 86400),
  (2, 1, 4, 'Receipt email shows the amount without currency', 'Receipts show no currency', 'The email says 150.000 with no IDR.', NULL, 'backlog', NULL, unixepoch() - 12 * 86400, unixepoch() - 11 * 86400);

-- 90 rolled-up days per monitor and heartbeat for the month chart.
INSERT INTO daily_summaries (source_kind, source_id, day, checks_total, checks_ok, downtime_s, avg_latency_ms)
WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 90)
SELECT 'monitor', m.id, date(unixepoch() - n.i * 86400, 'unixepoch'), 86400 / m.interval_s,
  86400 / m.interval_s - (CASE WHEN (n.i * 7 + m.id) % 23 = 0 THEN 3 ELSE 0 END), 0,
  80 + m.id * 12 + (n.i * 29 + m.id * 17) % 45
FROM monitors m, n
UNION ALL
SELECT 'heartbeat', h.id, date(unixepoch() - n.i * 86400, 'unixepoch'), 86400 / h.period_s,
  86400 / h.period_s - (CASE WHEN h.id = 2 AND n.i % 9 = 0 THEN 1 ELSE 0 END), 0, NULL
FROM heartbeats h, n;

-- What each project is, where people use it, and the public tabs it shows.
UPDATE projects SET description = 'The site and app our customers sign in to.', homepage_url = 'https://portal.example.com', show_changelog = 1 WHERE id = 1;
UPDATE projects SET description = 'Checkout, card payments and the bank callbacks behind them.', homepage_url = 'https://pay.example.com', show_changelog = 1 WHERE id = 2;
UPDATE projects SET description = 'Nightly loads and the query gateway for reports.', homepage_url = 'https://dwh.example.com' WHERE id = 3;

-- A supervisor reads every project and the admin pages, and changes nothing.
INSERT INTO users (id, email, role, password_hash, created_at) VALUES
  (4, 'supervisor@monita.test', 'supervisor', '$argon2id$v=19$m=19456,t=2,p=1$H5MnCFM1Yf3jchQQBWqLsQ$IJOegx6p9PvqbCdN8pb0xnxn0XeWAa73WOozfHbzl9U', unixepoch() - 30 * 86400);

-- The bank callback signs in with a sealed secret and sends a POST with a header.
INSERT INTO project_secrets (id, project_id, name, value, created_at, updated_at) VALUES
  (1, 2, 'BANK_CALLBACK_TOKEN', 'demo-not-a-real-token', unixepoch() - 80 * 86400, unixepoch() - 80 * 86400);
UPDATE monitors SET method = 'POST', headers = 'X-Env: production', body = '{"ping": true}', auth_kind = 'bearer', auth_secret_id = 1 WHERE id = 6;

-- Releases for the Changelog tabs, newest first on the public page.
INSERT INTO changelog_entries (project_id, version, released_on, title, sections, created_at, updated_at) VALUES
  (1, '2.4.0', date(unixepoch() - 3 * 86400, 'unixepoch'), 'Sign in with your company account',
    '[{"tag":"Added","items":["Sign in with your company Google or Microsoft account.","A remember-me box on the sign-in page."]},{"tag":"Fixed","items":["The login page no longer times out under load."]}]', unixepoch() - 3 * 86400, unixepoch() - 3 * 86400),
  (1, '2.3.1', date(unixepoch() - 18 * 86400, 'unixepoch'), 'Faster invoices page',
    '[{"tag":"Changed","items":["The invoices page loads in under a second, even with years of invoices."]},{"tag":"Fixed","items":["Downloaded invoices show the right tax number."]}]', unixepoch() - 18 * 86400, unixepoch() - 18 * 86400),
  (1, '2.3.0', date(unixepoch() - 41 * 86400, 'unixepoch'), 'A dark theme',
    '[{"tag":"Added","items":["A dark theme that follows your system setting.","Keyboard shortcuts for search and new tickets."]},{"tag":"Removed","items":["The old print view. Use Download PDF instead."]}]', unixepoch() - 41 * 86400, unixepoch() - 41 * 86400),
  (2, '5.1.0', date(unixepoch() - 6 * 86400, 'unixepoch'), 'QRIS payments',
    '[{"tag":"Added","items":["Customers can pay by QRIS at checkout."]},{"tag":"Changed","items":["Card payments retry once when the bank does not answer in time."]}]', unixepoch() - 6 * 86400, unixepoch() - 6 * 86400),
  (2, '5.0.2', date(unixepoch() - 27 * 86400, 'unixepoch'), 'Bank callback fixes',
    '[{"tag":"Fixed","items":["A late bank callback no longer marks a paid order as failed.","Refunds show in the order history at once."]}]', unixepoch() - 27 * 86400, unixepoch() - 27 * 86400);
