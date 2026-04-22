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
