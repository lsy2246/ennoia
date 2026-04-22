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
