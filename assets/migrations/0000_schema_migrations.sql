CREATE TABLE IF NOT EXISTS schema_migrations (
  logical_path TEXT PRIMARY KEY,
  applied_at TEXT NOT NULL
);
