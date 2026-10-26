-- A status update written through a project API key (v1.1 Q44) names the key, not a user. The
-- name is kept as text, so it reads without a join and survives the key's revoke. An update with
-- neither `author_id` nor `author_key` is Monita's own.
ALTER TABLE status_updates ADD COLUMN author_key TEXT;
