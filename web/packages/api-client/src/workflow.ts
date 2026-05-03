import { dispatchAction } from "./actions";
import type { ExecutionRun, ExecutionStep, RunOutput } from "./types";

export type WorkflowStageEvent = {
  id: string;
  run_id: string;
  from_stage?: string | null;
  to_stage: string;
  policy_rule_id?: string | null;
  reason?: string | null;
  at: string;
};

export type WorkflowGateVerdict = {
  gate_name: string;
  allow: boolean;
  severity: string;
  reason: string;
};

export type WorkflowDecisionSnapshot = {
  id: string;
  run_id?: string | null;
  task_id?: string | null;
  stage: string;
  signals_json: string;
  next_action: string;
  policy_rule_id: string;
  at: string;
};

export type WorkflowRunDetail = {
  run: ExecutionRun;
  tasks: ExecutionStep[];
  artifacts: RunOutput[];
  handoffs: Array<{
    id: string;
    from_lane_id: string;
    to_lane_id: string;
    from_agent_id?: string | null;
    to_agent_id?: string | null;
    summary: string;
    instructions: string;
    status: string;
    created_at: string;
  }>;
  stage_events: WorkflowStageEvent[];
  gate_verdicts: WorkflowGateVerdict[];
  decisions: WorkflowDecisionSnapshot[];
};

export async function getWorkflowRunDetail(runId: string) {
  return dispatchAction<WorkflowRunDetail>("run.get", { run_id: runId });
}
