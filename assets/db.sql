-- Ennoia SQLite initialization schema.
-- Runtime executes this file for a fresh database. Keep it self-contained;
-- do not call migration files from here.

PRAGMA journal_mode=WAL;
PRAGMA foreign_keys=ON;

CREATE TABLE IF NOT EXISTS agents (
  id TEXT PRIMARY KEY,
  display_name TEXT NOT NULL,
  kind TEXT NOT NULL,
  workspace_mode TEXT NOT NULL,
  default_model TEXT NOT NULL,
  skills_dir TEXT NOT NULL,
  workspace_dir TEXT NOT NULL,
  artifacts_dir TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS spaces (
  id TEXT PRIMARY KEY,
  display_name TEXT NOT NULL,
  description TEXT NOT NULL,
  primary_goal TEXT NOT NULL,
  mention_policy TEXT NOT NULL,
  default_agents_json TEXT NOT NULL DEFAULT '[]',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS workspace_profile (
  id TEXT PRIMARY KEY,
  display_name TEXT NOT NULL,
  locale TEXT NOT NULL,
  time_zone TEXT NOT NULL,
  default_space_id TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS instance_ui_preferences (
  id TEXT PRIMARY KEY,
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

CREATE TABLE IF NOT EXISTS conversations (
  id TEXT PRIMARY KEY,
  topology TEXT NOT NULL,
  owner_kind TEXT NOT NULL,
  owner_id TEXT NOT NULL,
  space_id TEXT,
  title TEXT NOT NULL,
  default_lane_id TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_conversations_updated_at
  ON conversations(updated_at DESC);

CREATE TABLE IF NOT EXISTS conversation_participants (
  conversation_id TEXT NOT NULL,
  participant_id TEXT NOT NULL,
  participant_kind TEXT NOT NULL,
  position INTEGER NOT NULL,
  PRIMARY KEY (conversation_id, participant_id)
);

CREATE INDEX IF NOT EXISTS idx_conversation_participants_order
  ON conversation_participants(conversation_id, position);

CREATE TABLE IF NOT EXISTS lanes (
  id TEXT PRIMARY KEY,
  conversation_id TEXT NOT NULL,
  space_id TEXT,
  name TEXT NOT NULL,
  lane_type TEXT NOT NULL,
  status TEXT NOT NULL,
  goal TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_lanes_conversation
  ON lanes(conversation_id, updated_at DESC);

CREATE TABLE IF NOT EXISTS lane_members (
  lane_id TEXT NOT NULL,
  participant_id TEXT NOT NULL,
  participant_kind TEXT NOT NULL,
  position INTEGER NOT NULL,
  PRIMARY KEY (lane_id, participant_id)
);

CREATE INDEX IF NOT EXISTS idx_lane_members_order
  ON lane_members(lane_id, position);

CREATE TABLE IF NOT EXISTS messages (
  id TEXT PRIMARY KEY,
  conversation_id TEXT NOT NULL,
  lane_id TEXT,
  sender TEXT NOT NULL,
  role TEXT NOT NULL,
  body TEXT NOT NULL,
  mentions_json TEXT NOT NULL DEFAULT '[]',
  created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_messages_conversation_time
  ON messages(conversation_id, created_at);

CREATE INDEX IF NOT EXISTS idx_messages_lane_time
  ON messages(lane_id, created_at);

CREATE TABLE IF NOT EXISTS handoffs (
  id TEXT PRIMARY KEY,
  from_lane_id TEXT NOT NULL,
  to_lane_id TEXT NOT NULL,
  from_agent_id TEXT,
  to_agent_id TEXT,
  summary TEXT NOT NULL,
  instructions TEXT NOT NULL,
  status TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_handoffs_from_lane
  ON handoffs(from_lane_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_handoffs_to_lane
  ON handoffs(to_lane_id, created_at DESC);

CREATE TABLE IF NOT EXISTS runs (
  id TEXT PRIMARY KEY,
  conversation_id TEXT NOT NULL,
  lane_id TEXT,
  owner_kind TEXT NOT NULL,
  owner_id TEXT NOT NULL,
  trigger TEXT NOT NULL,
  goal TEXT NOT NULL,
  stage TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_runs_conversation_time
  ON runs(conversation_id, created_at DESC);

CREATE TABLE IF NOT EXISTS tasks (
  id TEXT PRIMARY KEY,
  run_id TEXT NOT NULL,
  conversation_id TEXT NOT NULL,
  lane_id TEXT,
  task_kind TEXT NOT NULL,
  title TEXT NOT NULL,
  assigned_agent_id TEXT NOT NULL,
  status TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_tasks_run_time
  ON tasks(run_id, created_at);

CREATE TABLE IF NOT EXISTS artifacts (
  id TEXT PRIMARY KEY,
  owner_kind TEXT NOT NULL,
  owner_id TEXT NOT NULL,
  run_id TEXT,
  conversation_id TEXT,
  lane_id TEXT,
  artifact_kind TEXT NOT NULL,
  relative_path TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_artifacts_run_time
  ON artifacts(run_id, created_at);

CREATE TABLE IF NOT EXISTS run_stage_events (
  id TEXT PRIMARY KEY,
  run_id TEXT NOT NULL,
  from_stage TEXT,
  to_stage TEXT NOT NULL,
  policy_rule_id TEXT,
  reason TEXT,
  at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_run_stage_events_run
  ON run_stage_events(run_id, at);

CREATE TABLE IF NOT EXISTS decisions (
  id TEXT PRIMARY KEY,
  run_id TEXT,
  task_id TEXT,
  stage TEXT NOT NULL,
  signals_json TEXT NOT NULL,
  next_action TEXT NOT NULL,
  policy_rule_id TEXT NOT NULL,
  at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_decisions_run_time
  ON decisions(run_id, at);

CREATE TABLE IF NOT EXISTS gate_verdicts (
  id TEXT PRIMARY KEY,
  run_id TEXT,
  task_id TEXT,
  gate_name TEXT NOT NULL,
  verdict TEXT NOT NULL,
  reason TEXT,
  details_json TEXT NOT NULL DEFAULT '{}',
  at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_gate_verdicts_run_time
  ON gate_verdicts(run_id, at);

CREATE TABLE IF NOT EXISTS episodes (
  id TEXT PRIMARY KEY,
  owner_kind TEXT NOT NULL,
  owner_id TEXT NOT NULL,
  namespace TEXT NOT NULL,
  conversation_id TEXT,
  run_id TEXT,
  episode_kind TEXT NOT NULL,
  role TEXT,
  content TEXT NOT NULL,
  content_type TEXT NOT NULL DEFAULT 'text/plain',
  source_uri TEXT,
  entities_json TEXT NOT NULL DEFAULT '[]',
  tags_json TEXT NOT NULL DEFAULT '[]',
  importance REAL NOT NULL DEFAULT 0.2,
  occurred_at TEXT NOT NULL,
  ingested_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_episodes_ns_time
  ON episodes(owner_kind, owner_id, namespace, ingested_at);

CREATE INDEX IF NOT EXISTS idx_episodes_run
  ON episodes(run_id, ingested_at);

CREATE INDEX IF NOT EXISTS idx_episodes_conversation
  ON episodes(conversation_id, ingested_at);

CREATE TABLE IF NOT EXISTS memories (
  id TEXT PRIMARY KEY,
  owner_kind TEXT NOT NULL,
  owner_id TEXT NOT NULL,
  namespace TEXT NOT NULL,
  memory_kind TEXT NOT NULL,
  stability TEXT NOT NULL,
  status TEXT NOT NULL,
  superseded_by TEXT,
  title TEXT,
  content TEXT NOT NULL,
  summary TEXT,
  confidence REAL NOT NULL DEFAULT 0.6,
  importance REAL NOT NULL DEFAULT 0.5,
  valid_from TEXT,
  valid_to TEXT,
  sources_json TEXT NOT NULL DEFAULT '[]',
  tags_json TEXT NOT NULL DEFAULT '[]',
  entities_json TEXT NOT NULL DEFAULT '[]',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_memories_ns_status
  ON memories(owner_kind, owner_id, namespace, status);

CREATE INDEX IF NOT EXISTS idx_memories_stability
  ON memories(stability, updated_at);

CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(
  content,
  title,
  namespace,
  tags,
  entities,
  content='memories',
  content_rowid='rowid'
);

CREATE TRIGGER IF NOT EXISTS memories_ai AFTER INSERT ON memories BEGIN
  INSERT INTO memories_fts(rowid, content, title, namespace, tags, entities)
  VALUES (new.rowid, new.content, coalesce(new.title,''), new.namespace, new.tags_json, new.entities_json);
END;

CREATE TRIGGER IF NOT EXISTS memories_au AFTER UPDATE ON memories BEGIN
  INSERT INTO memories_fts(memories_fts, rowid, content, title, namespace, tags, entities)
  VALUES('delete', old.rowid, old.content, coalesce(old.title,''), old.namespace, old.tags_json, old.entities_json);
  INSERT INTO memories_fts(rowid, content, title, namespace, tags, entities)
  VALUES (new.rowid, new.content, coalesce(new.title,''), new.namespace, new.tags_json, new.entities_json);
END;

CREATE TRIGGER IF NOT EXISTS memories_ad AFTER DELETE ON memories BEGIN
  INSERT INTO memories_fts(memories_fts, rowid, content, title, namespace, tags, entities)
  VALUES('delete', old.rowid, old.content, coalesce(old.title,''), old.namespace, old.tags_json, old.entities_json);
END;

CREATE TABLE IF NOT EXISTS context_frames (
  id TEXT PRIMARY KEY,
  owner_kind TEXT NOT NULL,
  owner_id TEXT NOT NULL,
  namespace TEXT NOT NULL,
  layer TEXT NOT NULL,
  frame_kind TEXT NOT NULL,
  content TEXT NOT NULL,
  source_memory_ids_json TEXT NOT NULL DEFAULT '[]',
  budget_chars INTEGER,
  ttl_seconds INTEGER,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_context_frames_lookup
  ON context_frames(owner_kind, owner_id, namespace, layer, updated_at);

CREATE TABLE IF NOT EXISTS embeddings (
  id TEXT PRIMARY KEY,
  owner_kind TEXT NOT NULL,
  owner_id TEXT NOT NULL,
  target_kind TEXT NOT NULL,
  target_id TEXT NOT NULL,
  model TEXT NOT NULL,
  dims INTEGER NOT NULL,
  vector BLOB NOT NULL,
  content_hash TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_embeddings_target
  ON embeddings(target_kind, target_id);

CREATE TABLE IF NOT EXISTS remember_receipts (
  id TEXT PRIMARY KEY,
  owner_kind TEXT NOT NULL,
  owner_id TEXT NOT NULL,
  target_memory_id TEXT,
  action TEXT NOT NULL,
  policy_rule_id TEXT,
  details_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_remember_receipts_owner_time
  ON remember_receipts(owner_kind, owner_id, created_at);

CREATE TABLE IF NOT EXISTS recall_receipts (
  id TEXT PRIMARY KEY,
  owner_kind TEXT NOT NULL,
  owner_id TEXT NOT NULL,
  conversation_id TEXT,
  run_id TEXT,
  query_text TEXT,
  mode TEXT NOT NULL,
  memory_ids_json TEXT NOT NULL DEFAULT '[]',
  chars INTEGER NOT NULL DEFAULT 0,
  details_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_recall_receipts_owner_time
  ON recall_receipts(owner_kind, owner_id, created_at);

CREATE TABLE IF NOT EXISTS review_receipts (
  id TEXT PRIMARY KEY,
  owner_kind TEXT NOT NULL,
  owner_id TEXT NOT NULL,
  target_memory_id TEXT,
  action TEXT NOT NULL,
  old_status TEXT,
  new_status TEXT,
  reviewer TEXT,
  details_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_review_receipts_target_time
  ON review_receipts(target_memory_id, created_at);

CREATE TABLE IF NOT EXISTS extensions (
  id TEXT PRIMARY KEY,
  kind TEXT NOT NULL,
  version TEXT NOT NULL,
  install_dir TEXT NOT NULL,
  frontend_bundle TEXT,
  backend_entry TEXT,
  pages_json TEXT NOT NULL DEFAULT '[]',
  panels_json TEXT NOT NULL DEFAULT '[]',
  commands_json TEXT NOT NULL DEFAULT '[]',
  themes_json TEXT NOT NULL DEFAULT '[]',
  locales_json TEXT NOT NULL DEFAULT '[]',
  hooks_json TEXT NOT NULL DEFAULT '[]',
  providers_json TEXT NOT NULL DEFAULT '[]'
);

CREATE TABLE IF NOT EXISTS jobs (
  id TEXT PRIMARY KEY,
  owner_kind TEXT NOT NULL,
  owner_id TEXT NOT NULL,
  job_kind TEXT NOT NULL,
  schedule_kind TEXT NOT NULL,
  schedule_value TEXT NOT NULL,
  payload_json TEXT NOT NULL DEFAULT '{}',
  status TEXT NOT NULL,
  retry_count INTEGER NOT NULL DEFAULT 0,
  max_retries INTEGER NOT NULL DEFAULT 3,
  last_run_at TEXT,
  next_run_at TEXT,
  error TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_jobs_status_next
  ON jobs(status, next_run_at);

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

CREATE TABLE IF NOT EXISTS frontend_logs (
  id TEXT PRIMARY KEY,
  level TEXT NOT NULL,
  source TEXT NOT NULL,
  title TEXT NOT NULL,
  summary TEXT NOT NULL,
  details TEXT,
  at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_frontend_logs_at
  ON frontend_logs(at);

CREATE INDEX IF NOT EXISTS idx_frontend_logs_level
  ON frontend_logs(level);

