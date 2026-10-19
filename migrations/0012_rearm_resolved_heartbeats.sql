-- A heartbeat whose incident a tech lead resolved by hand before that re-armed it kept no
-- deadline: it showed Up, "Missed its deadline", and nothing watched it. Give each such
-- heartbeat a new deadline from now. One that is down (an open incident, or down with incidents
-- off) keeps waiting for its next ping, as before.
UPDATE heartbeats
SET deadline_at = CAST(strftime('%s', 'now') AS INTEGER) + period_s + grace_s
WHERE deadline_at IS NULL
  AND NOT EXISTS (
    SELECT 1 FROM incident_sources s
    WHERE s.source_kind = 'heartbeat' AND s.source_id = heartbeats.id
      AND (s.open_incident_id IS NOT NULL OR s.down_since IS NOT NULL)
  );
