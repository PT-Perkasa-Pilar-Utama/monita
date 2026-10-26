-- Pins (decision Q48): each user's favourite projects, and favourite monitors and heartbeats in a
-- project. A pin goes first in its list and is drawn first on the charts. At most 6 projects per
-- user, and 6 monitors and 6 heartbeats per project per user (`caps::PINS`). A pin goes with its
-- user or with what it pins.
CREATE TABLE project_pins (
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    pinned_at INTEGER NOT NULL,
    PRIMARY KEY (user_id, project_id)
);
CREATE INDEX project_pins_project ON project_pins (project_id);

CREATE TABLE monitor_pins (
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    monitor_id INTEGER NOT NULL REFERENCES monitors(id) ON DELETE CASCADE,
    pinned_at INTEGER NOT NULL,
    PRIMARY KEY (user_id, monitor_id)
);
CREATE INDEX monitor_pins_monitor ON monitor_pins (monitor_id);

CREATE TABLE heartbeat_pins (
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    heartbeat_id INTEGER NOT NULL REFERENCES heartbeats(id) ON DELETE CASCADE,
    pinned_at INTEGER NOT NULL,
    PRIMARY KEY (user_id, heartbeat_id)
);
CREATE INDEX heartbeat_pins_heartbeat ON heartbeat_pins (heartbeat_id);
