-- Monita ran on Turso until v1.1.0 and now runs on SQLite itself (decision Q47). Turso kept its
-- own AUTOINCREMENT counters in these tables beside `sqlite_sequence`, which it also kept up to
-- date, so SQLite goes on from the right ids. A database that SQLite created has none of them.
DROP TABLE IF EXISTS "__turso_internal_seq___turso_internal_autoincrement_monitors";
DROP TABLE IF EXISTS "__turso_internal_seq___turso_internal_autoincrement_heartbeats";
DROP TABLE IF EXISTS "__turso_internal_seq___turso_internal_autoincrement_locations";
