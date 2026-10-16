-- The nightly sweep deletes by time across every source; without these it scans both tables.
CREATE INDEX check_results_checked_at ON check_results (checked_at);
CREATE INDEX pings_received_at ON pings (received_at);
