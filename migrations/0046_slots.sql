-- Checks and pings summed per 15-minute UTC slot (slot = time / 900). The day and week charts,
-- the project charts and the sparklines read these instead of every raw row: 96 rows per source
-- per day instead of up to 2 880. A trigger fills them, so every insert path, seeds included,
-- keeps them right. Only this server's checks count, as in the charts (location_id IS NULL).
-- A failure inside a maintenance window counts as good, as in the charts (v1.1 Q46).
CREATE TABLE check_slots (
    monitor_id INTEGER NOT NULL REFERENCES monitors(id) ON DELETE CASCADE,
    slot INTEGER NOT NULL,
    total INTEGER NOT NULL,
    good INTEGER NOT NULL,
    timed INTEGER NOT NULL,
    ms_sum INTEGER NOT NULL,
    PRIMARY KEY (monitor_id, slot)
);

CREATE TABLE ping_slots (
    heartbeat_id INTEGER NOT NULL REFERENCES heartbeats(id) ON DELETE CASCADE,
    slot INTEGER NOT NULL,
    total INTEGER NOT NULL,
    good INTEGER NOT NULL,
    PRIMARY KEY (heartbeat_id, slot)
);

INSERT INTO check_slots (monitor_id, slot, total, good, timed, ms_sum)
SELECT monitor_id, checked_at / 900, count(*), sum(ok OR in_window), count(latency_ms), coalesce(sum(latency_ms), 0)
FROM check_results WHERE location_id IS NULL GROUP BY monitor_id, checked_at / 900;

INSERT INTO ping_slots (heartbeat_id, slot, total, good)
SELECT heartbeat_id, received_at / 900, count(*), sum(kind = 'ok' OR in_window)
FROM pings GROUP BY heartbeat_id, received_at / 900;

CREATE TRIGGER check_slots_add AFTER INSERT ON check_results WHEN NEW.location_id IS NULL
BEGIN
    INSERT INTO check_slots (monitor_id, slot, total, good, timed, ms_sum)
    VALUES (NEW.monitor_id, NEW.checked_at / 900, 1, NEW.ok OR NEW.in_window, NEW.latency_ms IS NOT NULL, coalesce(NEW.latency_ms, 0))
    ON CONFLICT (monitor_id, slot) DO UPDATE SET total = total + 1, good = good + excluded.good,
        timed = timed + excluded.timed, ms_sum = ms_sum + excluded.ms_sum;
END;

CREATE TRIGGER ping_slots_add AFTER INSERT ON pings
BEGIN
    INSERT INTO ping_slots (heartbeat_id, slot, total, good)
    VALUES (NEW.heartbeat_id, NEW.received_at / 900, 1, NEW.kind = 'ok' OR NEW.in_window)
    ON CONFLICT (heartbeat_id, slot) DO UPDATE SET total = total + 1, good = good + excluded.good;
END;
