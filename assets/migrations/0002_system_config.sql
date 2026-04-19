-- System configuration with hot-reload + change history.

CREATE TABLE IF NOT EXISTS system_config (
  key TEXT PRIMARY KEY,
  payload_json TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  version INTEGER NOT NULL DEFAULT 1,
  updated_by TEXT,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS system_config_history (
  id TEXT PRIMARY KEY,
  config_key TEXT NOT NULL,
  old_payload_json TEXT,
  new_payload_json TEXT NOT NULL,
  changed_by TEXT,
  changed_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_system_config_history_key_time
  ON system_config_history(config_key, changed_at DESC);
