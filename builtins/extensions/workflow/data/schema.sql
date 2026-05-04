CREATE TABLE IF NOT EXISTS runs (
  id TEXT PRIMARY KEY,
  payload_json TEXT NOT NULL,
  stage TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_workflow_runs_time
  ON runs(updated_at DESC);

CREATE TABLE IF NOT EXISTS plans (
  run_id TEXT PRIMARY KEY,
  payload_json TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_workflow_plans_time
  ON plans(updated_at DESC);

CREATE TABLE IF NOT EXISTS tasks (
  id TEXT PRIMARY KEY,
  run_id TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_workflow_tasks_run
  ON tasks(run_id, updated_at DESC);

CREATE TABLE IF NOT EXISTS artifacts (
  id TEXT PRIMARY KEY,
  run_id TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_workflow_artifacts_run
  ON artifacts(run_id, created_at DESC);

CREATE TABLE IF NOT EXISTS handoffs (
  id TEXT PRIMARY KEY,
  run_id TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_workflow_handoffs_run
  ON handoffs(run_id, created_at DESC);

CREATE TABLE IF NOT EXISTS run_stage_events (
  id TEXT PRIMARY KEY,
  run_id TEXT NOT NULL,
  from_stage TEXT,
  to_stage TEXT NOT NULL,
  policy_rule_id TEXT,
  reason TEXT NOT NULL,
  at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_workflow_stage_events_run
  ON run_stage_events(run_id, at);

CREATE TABLE IF NOT EXISTS decisions (
  id TEXT PRIMARY KEY,
  run_id TEXT NOT NULL,
  task_id TEXT,
  stage TEXT NOT NULL,
  signals_json TEXT NOT NULL,
  next_action TEXT NOT NULL,
  policy_rule_id TEXT,
  at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_workflow_decisions_run
  ON decisions(run_id, at);

CREATE TABLE IF NOT EXISTS gate_verdicts (
  id TEXT PRIMARY KEY,
  run_id TEXT NOT NULL,
  task_id TEXT,
  gate_name TEXT NOT NULL,
  verdict TEXT NOT NULL,
  reason TEXT NOT NULL,
  details_json TEXT NOT NULL,
  at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_workflow_gate_verdicts_run
  ON gate_verdicts(run_id, at);

CREATE TABLE IF NOT EXISTS drafts (
  id TEXT PRIMARY KEY,
  conversation_id TEXT NOT NULL,
  agent_id TEXT NOT NULL,
  status TEXT NOT NULL,
  goal TEXT NOT NULL,
  summary TEXT NOT NULL,
  source_message_id TEXT,
  record_id TEXT,
  latest_revision INTEGER NOT NULL DEFAULT 1,
  payload_json TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_workflow_drafts_conversation_agent
  ON drafts(conversation_id, agent_id, updated_at DESC);

CREATE TABLE IF NOT EXISTS draft_revisions (
  id TEXT PRIMARY KEY,
  draft_id TEXT NOT NULL,
  revision INTEGER NOT NULL,
  goal TEXT NOT NULL,
  summary TEXT NOT NULL,
  source_message_id TEXT,
  payload_json TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_workflow_draft_revisions_draft
  ON draft_revisions(draft_id, revision DESC);

CREATE TABLE IF NOT EXISTS conversation_message_receipts (
  conversation_id TEXT NOT NULL,
  message_id TEXT NOT NULL,
  agent_id TEXT NOT NULL,
  status TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  PRIMARY KEY (conversation_id, message_id, agent_id)
);

CREATE INDEX IF NOT EXISTS idx_workflow_message_receipts_conversation
  ON conversation_message_receipts(conversation_id, updated_at DESC);
