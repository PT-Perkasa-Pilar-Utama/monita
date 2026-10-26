-- Grows the parity fixture to a week of load: portal gets 100 monitors every 30 s, payments 100
-- monitors every 60 s and 100 heartbeats pinging every minute. About 3 million check rows and
-- 1 million pings, the upper end of the 7-day raw retention. Checks and deadlines stay far out,
-- so the servers under test only read. Run with sqlite3 on a copy of the fixture.
BEGIN;
INSERT INTO monitors (project_id, name, url, interval_s, next_check_at, is_public)
WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 100)
SELECT p.id, 'Load ' || i, 'http://127.0.0.1:1/' || i, 60, 4000000000, 1
FROM n, projects p WHERE p.slug = 'payments' AND i <= 100 - (SELECT count(*) FROM monitors WHERE project_id = p.id);
INSERT INTO monitors (project_id, name, url, interval_s, next_check_at, is_public)
WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 100)
SELECT p.id, 'Portal load ' || i, 'http://127.0.0.1:1/p' || i, 30, 4000000000, 1
FROM n, projects p WHERE p.slug = 'portal' AND i <= 100 - (SELECT count(*) FROM monitors WHERE project_id = p.id);
INSERT INTO heartbeats (project_id, name, token, period_s, grace_s, last_ping_at, deadline_at)
WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 100)
SELECT p.id, 'Load job ' || i, 'perftoken' || i, 60, 60, unixepoch(), 4000000000
FROM n, projects p WHERE p.slug = 'payments' AND i <= 100 - (SELECT count(*) FROM heartbeats WHERE project_id = p.id);
UPDATE monitors SET next_check_at = 4000000000;
UPDATE heartbeats SET deadline_at = 4000000000 WHERE deadline_at IS NOT NULL;
DELETE FROM check_results;
DELETE FROM check_slots;
DELETE FROM pings;
DELETE FROM ping_slots;
-- The slot triggers fill check_slots and ping_slots as these rows go in. 1 check in 200 fails.
INSERT INTO check_results (monitor_id, checked_at, ok, status_code, latency_ms)
WITH RECURSIVE t(k) AS (SELECT 0 UNION ALL SELECT k + 1 FROM t WHERE k < 20159)
SELECT m.id, unixepoch() - 604800 + t.k * m.interval_s + (m.id % 30),
  CASE WHEN (t.k * 7 + m.id) % 200 = 0 THEN 0 ELSE 1 END, 200, 40 + (t.k * 13 + m.id) % 300
FROM monitors m JOIN t ON t.k * m.interval_s < 604800;
INSERT INTO pings (heartbeat_id, received_at, kind)
WITH RECURSIVE t(k) AS (SELECT 0 UNION ALL SELECT k + 1 FROM t WHERE k < 10079)
SELECT h.id, unixepoch() - 604800 + t.k * 60 + (h.id % 50), CASE WHEN (t.k + h.id) % 500 = 0 THEN 'fail' ELSE 'ok' END
FROM heartbeats h JOIN t;
COMMIT;
PRAGMA wal_checkpoint(TRUNCATE);
