-- A user's own picture (v1.1 Q31), kept as a project icon is: a PNG or WebP of a few KB, in its
-- own table so user lists never read it. avatar_at marks that one exists and versions its URL.
CREATE TABLE user_avatars (
  user_id INTEGER PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
  mime TEXT NOT NULL CHECK (mime IN ('image/png', 'image/webp')),
  data BLOB NOT NULL
);
ALTER TABLE users ADD COLUMN avatar_at INTEGER;
