-- A tech lead may reclassify an incident (v1.1 Q46): `planned` for maintenance or a pause that
-- came too late, `false_alarm` for a check that was wrong. A reclassified incident leaves
-- uptime, the charts and the public page; the team still sees it. NULL counts as usual.
-- `excluded_by` is the name shown beside it: the user's email or "API key <name>".
ALTER TABLE incidents ADD COLUMN excluded TEXT CHECK (excluded IN ('planned', 'false_alarm'));
ALTER TABLE incidents ADD COLUMN excluded_at INTEGER;
ALTER TABLE incidents ADD COLUMN excluded_by TEXT;
