CREATE TABLE IF NOT EXISTS agents (
  id TEXT PRIMARY KEY,
  display_name TEXT NOT NULL,
  kind TEXT NOT NULL,
  workspace_mode TEXT NOT NULL,
  default_model TEXT NOT NULL,
  skills_dir TEXT NOT NULL,
  workspace_dir TEXT NOT NULL,
  artifacts_dir TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS spaces (
  id TEXT PRIMARY KEY,
  display_name TEXT NOT NULL,
  mention_policy TEXT NOT NULL,
  default_agents_json TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS runs (
  id TEXT PRIMARY KEY,
  owner_kind TEXT NOT NULL,
  owner_id TEXT NOT NULL,
  thread_id TEXT NOT NULL,
  trigger TEXT NOT NULL,
  status TEXT NOT NULL,
  goal TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS tasks (
  id TEXT PRIMARY KEY,
  run_id TEXT NOT NULL,
  title TEXT NOT NULL,
  assigned_agent_id TEXT NOT NULL,
  status TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS memories (
  id TEXT PRIMARY KEY,
  owner_kind TEXT NOT NULL,
  owner_id TEXT NOT NULL,
  memory_kind TEXT NOT NULL,
  source TEXT NOT NULL,
  content TEXT NOT NULL,
  summary TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS jobs (
  id TEXT PRIMARY KEY,
  owner_kind TEXT NOT NULL,
  owner_id TEXT NOT NULL,
  schedule_kind TEXT NOT NULL,
  schedule_value TEXT NOT NULL,
  description TEXT NOT NULL,
  status TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS extensions (
  id TEXT PRIMARY KEY,
  kind TEXT NOT NULL,
  version TEXT NOT NULL,
  install_dir TEXT NOT NULL,
  frontend_bundle TEXT,
  backend_entry TEXT,
  pages_json TEXT NOT NULL,
  panels_json TEXT NOT NULL,
  commands_json TEXT NOT NULL,
  themes_json TEXT NOT NULL,
  hooks_json TEXT NOT NULL,
  providers_json TEXT NOT NULL
);
