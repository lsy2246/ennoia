ALTER TABLE extensions ADD COLUMN locales_json TEXT NOT NULL DEFAULT '[]';

CREATE TABLE IF NOT EXISTS user_ui_preferences (
  user_id TEXT PRIMARY KEY,
  locale TEXT,
  theme_id TEXT,
  time_zone TEXT,
  date_style TEXT,
  density TEXT,
  motion TEXT,
  version INTEGER NOT NULL DEFAULT 1,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS space_ui_preferences (
  space_id TEXT PRIMARY KEY,
  locale TEXT,
  theme_id TEXT,
  time_zone TEXT,
  date_style TEXT,
  density TEXT,
  motion TEXT,
  version INTEGER NOT NULL DEFAULT 1,
  updated_at TEXT NOT NULL
);
