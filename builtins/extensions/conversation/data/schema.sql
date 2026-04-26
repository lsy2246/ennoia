CREATE TABLE IF NOT EXISTS conversations (
  id TEXT PRIMARY KEY,
  topology TEXT NOT NULL,
  owner_kind TEXT NOT NULL,
  owner_id TEXT NOT NULL,
  space_id TEXT,
  title TEXT NOT NULL,
  active_branch_id TEXT,
  default_lane_id TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_conversations_updated_at
  ON conversations(updated_at DESC);

CREATE TABLE IF NOT EXISTS conversation_participants (
  conversation_id TEXT NOT NULL,
  participant_id TEXT NOT NULL,
  position INTEGER NOT NULL,
  PRIMARY KEY (conversation_id, participant_id)
);

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
  position INTEGER NOT NULL,
  PRIMARY KEY (lane_id, participant_id)
);

CREATE TABLE IF NOT EXISTS branches (
  id TEXT PRIMARY KEY,
  conversation_id TEXT NOT NULL,
  name TEXT NOT NULL,
  kind TEXT NOT NULL,
  status TEXT NOT NULL,
  parent_branch_id TEXT,
  source_message_id TEXT,
  source_checkpoint_id TEXT,
  inherit_mode TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_branches_conversation
  ON branches(conversation_id, updated_at DESC);

CREATE TABLE IF NOT EXISTS checkpoints (
  id TEXT PRIMARY KEY,
  conversation_id TEXT NOT NULL,
  branch_id TEXT NOT NULL,
  message_id TEXT,
  kind TEXT NOT NULL,
  label TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_checkpoints_conversation
  ON checkpoints(conversation_id, created_at DESC);

CREATE TABLE IF NOT EXISTS messages (
  id TEXT PRIMARY KEY,
  conversation_id TEXT NOT NULL,
  branch_id TEXT,
  lane_id TEXT,
  sender TEXT NOT NULL,
  role TEXT NOT NULL,
  body TEXT NOT NULL,
  mentions_json TEXT NOT NULL,
  reply_to_message_id TEXT,
  rewrite_from_message_id TEXT,
  created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_messages_conversation_time
  ON messages(conversation_id, created_at);

CREATE INDEX IF NOT EXISTS idx_messages_branch_time
  ON messages(branch_id, created_at);
