-- The text a job sends with /ping/{token}/fail, up to 1 KB (api-specs 04 section 4.2).
ALTER TABLE pings ADD COLUMN cause TEXT;
