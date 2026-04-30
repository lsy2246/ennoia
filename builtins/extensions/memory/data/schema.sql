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
  review_state TEXT NOT NULL DEFAULT 'approved',
  superseded_by TEXT,
  supersedes TEXT,
  title TEXT,
  content TEXT NOT NULL,
  summary TEXT,
  confidence REAL NOT NULL DEFAULT 0.6,
  importance REAL NOT NULL DEFAULT 0.5,
  valid_from TEXT,
  valid_to TEXT,
  sources_json TEXT NOT NULL DEFAULT '[]',
  evidence_refs_json TEXT NOT NULL DEFAULT '[]',
  tags_json TEXT NOT NULL DEFAULT '[]',
  entities_json TEXT NOT NULL DEFAULT '[]',
  active_count INTEGER NOT NULL DEFAULT 0,
  last_used_at TEXT,
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

CREATE TABLE IF NOT EXISTS embeddings (
  id TEXT PRIMARY KEY,
  owner_kind TEXT NOT NULL,
  owner_id TEXT NOT NULL,
  target_type TEXT NOT NULL,
  target_id TEXT NOT NULL,
  model TEXT NOT NULL,
  dims INTEGER NOT NULL,
  vector BLOB NOT NULL,
  created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_embeddings_target
  ON embeddings(target_type, target_id);

CREATE TABLE IF NOT EXISTS embeddings_vec (
  memory_id TEXT PRIMARY KEY,
  embedding BLOB NOT NULL,
  updated_at TEXT NOT NULL
);

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
  ON context_frames(owner_kind, owner_id, namespace, layer, updated_at DESC);

CREATE TABLE IF NOT EXISTS context_segments (
  id TEXT PRIMARY KEY,
  owner_kind TEXT NOT NULL,
  owner_id TEXT NOT NULL,
  namespace TEXT NOT NULL,
  conversation_id TEXT,
  lane_id TEXT,
  start_episode_id TEXT,
  end_episode_id TEXT,
  start_ingested_at TEXT,
  end_ingested_at TEXT,
  episode_ids_json TEXT NOT NULL DEFAULT '[]',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_context_segments_lookup
  ON context_segments(owner_kind, owner_id, namespace, end_ingested_at);

CREATE TABLE IF NOT EXISTS context_artifacts (
  id TEXT PRIMARY KEY,
  owner_kind TEXT NOT NULL,
  owner_id TEXT NOT NULL,
  namespace TEXT NOT NULL,
  conversation_id TEXT,
  lane_id TEXT,
  layer TEXT NOT NULL,
  artifact_kind TEXT NOT NULL,
  derived_kind TEXT,
  content TEXT NOT NULL,
  source_memory_ids_json TEXT NOT NULL DEFAULT '[]',
  source_episode_ids_json TEXT NOT NULL DEFAULT '[]',
  source_refs_json TEXT NOT NULL DEFAULT '[]',
  budget_chars INTEGER,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_context_artifacts_lookup
  ON context_artifacts(owner_kind, owner_id, namespace, layer, updated_at);

CREATE TABLE IF NOT EXISTS active_views (
  owner_kind TEXT NOT NULL,
  owner_id TEXT NOT NULL,
  namespace TEXT NOT NULL,
  layer TEXT NOT NULL,
  current_artifact_id TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  PRIMARY KEY (owner_kind, owner_id, namespace, layer)
);

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

CREATE TABLE IF NOT EXISTS injection_receipts (
  id TEXT PRIMARY KEY,
  owner_kind TEXT NOT NULL,
  owner_id TEXT NOT NULL,
  conversation_id TEXT,
  lane_id TEXT,
  query_text TEXT,
  artifact_ids_json TEXT NOT NULL DEFAULT '[]',
  memory_ids_json TEXT NOT NULL DEFAULT '[]',
  chars INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS commit_receipts (
  id TEXT PRIMARY KEY,
  owner_kind TEXT NOT NULL,
  owner_id TEXT NOT NULL,
  conversation_id TEXT,
  lane_id TEXT,
  namespace TEXT NOT NULL,
  promoted_json TEXT NOT NULL DEFAULT '[]',
  created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS flush_receipts (
  id TEXT PRIMARY KEY,
  owner_kind TEXT NOT NULL,
  owner_id TEXT NOT NULL,
  conversation_id TEXT,
  lane_id TEXT,
  namespace TEXT,
  drained_count INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS gm_nodes (
  id TEXT PRIMARY KEY,
  owner_kind TEXT NOT NULL,
  owner_id TEXT NOT NULL,
  namespace TEXT NOT NULL,
  node_type TEXT NOT NULL,
  name TEXT NOT NULL,
  description TEXT NOT NULL DEFAULT '',
  content TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'active',
  validated_count INTEGER NOT NULL DEFAULT 1,
  confidence REAL NOT NULL DEFAULT 0.6,
  source_episode_ids_json TEXT NOT NULL DEFAULT '[]',
  source_memory_ids_json TEXT NOT NULL DEFAULT '[]',
  source_refs_json TEXT NOT NULL DEFAULT '[]',
  community_id TEXT,
  pagerank REAL NOT NULL DEFAULT 0,
  freshness_score REAL NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_gm_nodes_owner_ns_type_name
  ON gm_nodes(owner_kind, owner_id, namespace, node_type, name);

CREATE TABLE IF NOT EXISTS gm_edges (
  id TEXT PRIMARY KEY,
  owner_kind TEXT NOT NULL,
  owner_id TEXT NOT NULL,
  from_node_id TEXT NOT NULL,
  to_node_id TEXT NOT NULL,
  edge_type TEXT NOT NULL,
  instruction TEXT NOT NULL,
  condition TEXT,
  session_key TEXT,
  evidence_refs_json TEXT NOT NULL DEFAULT '[]',
  weight REAL NOT NULL DEFAULT 1.0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_gm_edges_from_to_type
  ON gm_edges(from_node_id, to_node_id, edge_type);

CREATE TABLE IF NOT EXISTS gm_vectors (
  node_id TEXT PRIMARY KEY,
  model TEXT NOT NULL,
  dims INTEGER NOT NULL,
  vector BLOB NOT NULL,
  content_hash TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS gm_communities (
  id TEXT PRIMARY KEY,
  owner_kind TEXT NOT NULL,
  owner_id TEXT NOT NULL,
  namespace TEXT NOT NULL,
  summary TEXT NOT NULL,
  node_count INTEGER NOT NULL DEFAULT 0,
  embedding BLOB,
  cohesion REAL NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS gm_jobs (
  id TEXT PRIMARY KEY,
  owner_kind TEXT,
  owner_id TEXT,
  job_type TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'pending',
  payload_json TEXT NOT NULL DEFAULT '{}',
  error TEXT,
  retry_count INTEGER NOT NULL DEFAULT 0,
  max_retries INTEGER NOT NULL DEFAULT 5,
  last_run_at TEXT,
  next_run_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS graph_recall_receipts (
  id TEXT PRIMARY KEY,
  owner_kind TEXT NOT NULL,
  owner_id TEXT NOT NULL,
  conversation_id TEXT,
  lane_id TEXT,
  query_text TEXT NOT NULL,
  mode TEXT NOT NULL,
  truth_memory_ids_json TEXT NOT NULL DEFAULT '[]',
  graph_node_ids_json TEXT NOT NULL DEFAULT '[]',
  graph_edge_ids_json TEXT NOT NULL DEFAULT '[]',
  community_ids_json TEXT NOT NULL DEFAULT '[]',
  ranked_json TEXT NOT NULL DEFAULT '[]',
  injected_chars INTEGER NOT NULL DEFAULT 0,
  details_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL
);

