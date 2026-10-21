-- qa seed. Loaded by POST /admin/reset-state {"seed":"qa"}; the route does not exist in prod.
-- Every account uses the password in seeds/README.md.
INSERT INTO users (email, role, password_hash) VALUES
  ('admin@monita.test', 'super_admin', '$argon2id$v=19$m=19456,t=2,p=1$H5MnCFM1Yf3jchQQBWqLsQ$IJOegx6p9PvqbCdN8pb0xnxn0XeWAa73WOozfHbzl9U'),
  ('tl@monita.test', 'tl', '$argon2id$v=19$m=19456,t=2,p=1$H5MnCFM1Yf3jchQQBWqLsQ$IJOegx6p9PvqbCdN8pb0xnxn0XeWAa73WOozfHbzl9U');
-- A supervisor: reads every project and the admin pages, changes nothing.
INSERT INTO users (email, role, password_hash) VALUES ('supervisor@monita.test', 'supervisor', '$argon2id$v=19$m=19456,t=2,p=1$H5MnCFM1Yf3jchQQBWqLsQ$IJOegx6p9PvqbCdN8pb0xnxn0XeWAa73WOozfHbzl9U');
-- A project with history, for the Past uptime arrows: its public monitor is 200 days old. It never
-- runs a check (next_check_at is far ahead), so the suite sends no traffic for it. Ids start at
-- 900: tests that reset to this seed may create id 1 themselves.
INSERT INTO projects (id, owner_id, name, slug, created_at) VALUES (900, 1, 'Archive', 'qa-archive', unixepoch() - 200 * 86400);
INSERT INTO alert_cards (project_id, kind, name, critical, warning) VALUES
  (900, 'owner_email', 'Owner email', 1, 0), (900, 'owner_email', 'Owner email', 0, 1);
INSERT INTO monitors (id, project_id, name, url, interval_s, expected_status, severity, is_public, next_check_at, created_at) VALUES
  (900, 900, 'Archive site', 'http://127.0.0.1:9/', 300, '2xx', 'warning', 1, unixepoch() + 3650 * 86400, unixepoch() - 200 * 86400);
