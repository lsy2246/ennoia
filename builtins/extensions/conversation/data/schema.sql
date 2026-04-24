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

CREATE TABLE IF NOT EXISTS messages (
  id TEXT PRIMARY KEY,
  conversation_id TEXT NOT NULL,
  lane_id TEXT,
  sender TEXT NOT NULL,
  role TEXT NOT NULL,
  body TEXT NOT NULL,
  mentions_json TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_messages_conversation_time
  ON messages(conversation_id, created_at);
