-- v1.2 Q59: each project's weekly and monthly summary report, sent as a PDF to a list of
-- addresses. A missing row is a schedule that is off. `last_from` is the start of the period sent
-- last, written in the same transaction that queues the emails, so a period goes out once.
CREATE TABLE report_schedules (
    project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    cadence TEXT NOT NULL CHECK (cadence IN ('weekly', 'monthly')),
    enabled INTEGER NOT NULL DEFAULT 0 CHECK (enabled IN (0, 1)),
    emails TEXT NOT NULL DEFAULT '',
    last_from INTEGER,
    PRIMARY KEY (project_id, cadence)
);

-- An email may carry one PDF. Delivery clears it with the body once the row is finished.
ALTER TABLE outbox ADD COLUMN attachment BLOB;
ALTER TABLE outbox ADD COLUMN attachment_name TEXT;
