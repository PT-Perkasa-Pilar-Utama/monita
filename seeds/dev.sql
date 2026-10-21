-- dev seed. Loaded by POST /admin/reset-state {"seed":"dev"}; the route does not exist in prod.
-- Both accounts use the password in seeds/README.md.
INSERT INTO users (email, role, password_hash) VALUES
  ('admin@monita.test', 'super_admin', '$argon2id$v=19$m=19456,t=2,p=1$H5MnCFM1Yf3jchQQBWqLsQ$IJOegx6p9PvqbCdN8pb0xnxn0XeWAa73WOozfHbzl9U'),
  ('tl@monita.test', 'tl', '$argon2id$v=19$m=19456,t=2,p=1$H5MnCFM1Yf3jchQQBWqLsQ$IJOegx6p9PvqbCdN8pb0xnxn0XeWAa73WOozfHbzl9U');
INSERT INTO projects (owner_id, name, slug) VALUES (2, 'Demo', 'demo');
INSERT INTO alert_routes (project_id, severity, use_webhook) VALUES (1, 'critical', 0), (1, 'warning', 0);
-- A supervisor: reads every project and the admin pages, changes nothing.
INSERT INTO users (email, role, password_hash) VALUES ('supervisor@monita.test', 'supervisor', '$argon2id$v=19$m=19456,t=2,p=1$H5MnCFM1Yf3jchQQBWqLsQ$IJOegx6p9PvqbCdN8pb0xnxn0XeWAa73WOozfHbzl9U');
