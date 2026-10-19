-- Two-factor sign-in with time-based codes (v1.1 decision Q19). The secret is stored as entered,
-- like project secrets (SEC-23): Monita must compute the code. totp_pending holds a secret shown
-- but not yet confirmed with a code. totp_step is the last step a code was used for, so no code
-- works twice.
ALTER TABLE users ADD COLUMN totp_secret TEXT;
ALTER TABLE users ADD COLUMN totp_pending TEXT;
ALTER TABLE users ADD COLUMN totp_step INTEGER;

-- Ten per user, shown once. Only the SHA-256 of each is stored (SEC-07).
CREATE TABLE recovery_codes (
  id INTEGER PRIMARY KEY,
  user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  code_hash TEXT NOT NULL UNIQUE,
  used_at INTEGER
);
CREATE INDEX recovery_codes_user ON recovery_codes (user_id);

-- The step between a right password and a right code. The raw token goes to the browser once;
-- only its SHA-256 is stored. Five wrong codes or five minutes end it.
CREATE TABLE sign_in_challenges (
  token_hash TEXT PRIMARY KEY,
  user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  expires_at INTEGER NOT NULL,
  attempts INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX sign_in_challenges_expiry ON sign_in_challenges (expires_at);
