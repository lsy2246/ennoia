PRAGMA foreign_keys=OFF;

DROP TABLE IF EXISTS users;
DROP TABLE IF EXISTS sessions;
DROP TABLE IF EXISTS api_keys;
DROP TABLE IF EXISTS threads;
DROP TABLE IF EXISTS messages;
DROP TABLE IF EXISTS conversation_participants;
DROP TABLE IF EXISTS conversations;
DROP TABLE IF EXISTS lane_members;
DROP TABLE IF EXISTS lanes;
DROP TABLE IF EXISTS handoffs;
DROP TABLE IF EXISTS runs;
DROP TABLE IF EXISTS tasks;
DROP TABLE IF EXISTS artifacts;
DROP TABLE IF EXISTS run_stage_events;
DROP TABLE IF EXISTS decisions;
DROP TABLE IF EXISTS gate_verdicts;
DROP TABLE IF EXISTS workspace_profile;
DROP TABLE IF EXISTS instance_ui_preferences;
DROP TABLE IF EXISTS space_ui_preferences;
DROP TABLE IF EXISTS spaces;

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

PRAGMA foreign_keys=ON;
