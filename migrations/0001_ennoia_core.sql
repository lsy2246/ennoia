PRAGMA journal_mode=WAL;
PRAGMA foreign_keys=ON;

-- ========== Identity & structure ==========
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
  mention_policy TEXT NOT NULL,
  default_agents_json TEXT NOT NULL DEFAULT '[]',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS threads (
  id TEXT PRIMARY KEY,
  thread_kind TEXT NOT NULL,
  owner_kind TEXT NOT NULL,
  owner_id TEXT NOT NULL,
  space_id TEXT,
  title TEXT NOT NULL,
  participants_json TEXT NOT NULL DEFAULT '[]',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS messages (
  id TEXT PRIMARY KEY,
  thread_id TEXT NOT NULL,
  sender TEXT NOT NULL,
  role TEXT NOT NULL,
  body TEXT NOT NULL,
  mentions_json TEXT NOT NULL DEFAULT '[]',
  created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_messages_thread_time ON messages(thread_id, created_at);

-- ========== Run / Task / Stage ==========
CREATE TABLE IF NOT EXISTS runs (
  id TEXT PRIMARY KEY,
  thread_id TEXT NOT NULL,
  owner_kind TEXT NOT NULL,
  owner_id TEXT NOT NULL,
  trigger TEXT NOT NULL,
  goal TEXT NOT NULL,
  stage TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS tasks (
  id TEXT PRIMARY KEY,
  run_id TEXT NOT NULL,
  task_kind TEXT NOT NULL,
  title TEXT NOT NULL,
  assigned_agent_id TEXT NOT NULL,
  status TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS run_stage_events (
  id TEXT PRIMARY KEY,
  run_id TEXT NOT NULL,
  from_stage TEXT,
  to_stage TEXT NOT NULL,
  policy_rule_id TEXT,
  reason TEXT,
  at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_run_stage_events_run ON run_stage_events(run_id, at);

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

CREATE INDEX IF NOT EXISTS idx_decisions_run_time ON decisions(run_id, at);

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

CREATE INDEX IF NOT EXISTS idx_gate_verdicts_run_time ON gate_verdicts(run_id, at);

-- ========== Memory ==========
CREATE TABLE IF NOT EXISTS episodes (
  id TEXT PRIMARY KEY,
  owner_kind TEXT NOT NULL,
  owner_id TEXT NOT NULL,
  namespace TEXT NOT NULL,
  thread_id TEXT,
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

CREATE INDEX IF NOT EXISTS idx_episodes_ns_time ON episodes(owner_kind, owner_id, namespace, ingested_at);
CREATE INDEX IF NOT EXISTS idx_episodes_run ON episodes(run_id, ingested_at);
CREATE INDEX IF NOT EXISTS idx_episodes_thread ON episodes(thread_id, ingested_at);

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

CREATE INDEX IF NOT EXISTS idx_memories_ns_status ON memories(owner_kind, owner_id, namespace, status);
CREATE INDEX IF NOT EXISTS idx_memories_stability ON memories(stability, updated_at);

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

CREATE INDEX IF NOT EXISTS idx_context_frames_lookup ON context_frames(owner_kind, owner_id, namespace, layer, updated_at);

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

CREATE INDEX IF NOT EXISTS idx_embeddings_target ON embeddings(target_kind, target_id);

-- ========== Audit receipts ==========
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

CREATE INDEX IF NOT EXISTS idx_remember_receipts_owner_time ON remember_receipts(owner_kind, owner_id, created_at);

CREATE TABLE IF NOT EXISTS recall_receipts (
  id TEXT PRIMARY KEY,
  owner_kind TEXT NOT NULL,
  owner_id TEXT NOT NULL,
  thread_id TEXT,
  run_id TEXT,
  query_text TEXT,
  mode TEXT NOT NULL,
  memory_ids_json TEXT NOT NULL DEFAULT '[]',
  chars INTEGER NOT NULL DEFAULT 0,
  details_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_recall_receipts_owner_time ON recall_receipts(owner_kind, owner_id, created_at);

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

CREATE INDEX IF NOT EXISTS idx_review_receipts_target_time ON review_receipts(target_memory_id, created_at);

-- ========== Artifacts & extensions ==========
CREATE TABLE IF NOT EXISTS artifacts (
  id TEXT PRIMARY KEY,
  owner_kind TEXT NOT NULL,
  owner_id TEXT NOT NULL,
  run_id TEXT,
  artifact_kind TEXT NOT NULL,
  relative_path TEXT NOT NULL,
  created_at TEXT NOT NULL
);

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
  hooks_json TEXT NOT NULL DEFAULT '[]',
  providers_json TEXT NOT NULL DEFAULT '[]'
);

-- ========== Scheduler ==========
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

CREATE INDEX IF NOT EXISTS idx_jobs_status_next ON jobs(status, next_run_at);
