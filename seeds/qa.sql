-- qa seed. Loaded by POST /admin/reset-state {"seed":"qa"}; the route does not exist in prod.
-- Every account uses the password in seeds/README.md.
INSERT INTO users (email, role, password_hash) VALUES
  ('admin@monita.test', 'super_admin', '$argon2id$v=19$m=19456,t=2,p=1$H5MnCFM1Yf3jchQQBWqLsQ$IJOegx6p9PvqbCdN8pb0xnxn0XeWAa73WOozfHbzl9U'),
  ('tl@monita.test', 'tl', '$argon2id$v=19$m=19456,t=2,p=1$H5MnCFM1Yf3jchQQBWqLsQ$IJOegx6p9PvqbCdN8pb0xnxn0XeWAa73WOozfHbzl9U');
-- A supervisor: reads every project and the admin pages, changes nothing.
INSERT INTO users (email, role, password_hash) VALUES ('supervisor@monita.test', 'supervisor', '$argon2id$v=19$m=19456,t=2,p=1$H5MnCFM1Yf3jchQQBWqLsQ$IJOegx6p9PvqbCdN8pb0xnxn0XeWAa73WOozfHbzl9U');
