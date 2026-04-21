CREATE TABLE IF NOT EXISTS frontend_logs (
  id TEXT PRIMARY KEY,
  level TEXT NOT NULL,
  source TEXT NOT NULL,
  title TEXT NOT NULL,
  summary TEXT NOT NULL,
  details TEXT,
  at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_frontend_logs_at ON frontend_logs (at);
CREATE INDEX IF NOT EXISTS idx_frontend_logs_level ON frontend_logs (level);
