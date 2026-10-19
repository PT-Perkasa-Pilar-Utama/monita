-- The last 4 characters of a ref ID, so the team can tell ref IDs apart. Only the hash of the
-- whole value is kept; 4 of its 26 random characters leave 110 bits unknown. NULL for older ones.
ALTER TABLE ref_ids ADD COLUMN tail TEXT;
