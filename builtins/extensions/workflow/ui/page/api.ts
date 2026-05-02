import { fetchJson } from "@ennoia/api-client";

export type WorkflowWorkspaceSummary = {
  runs_total: number;
  runs_active: number;
  runs_blocked: number;
  runs_completed: number;
  runs_failed: number;
  tasks_total: number;
  artifacts_total: number;
  handoffs_total: number;
  decisions_total: number;
  gate_verdicts_total: number;
  latest_run_id?: string | null;
  latest_run_stage?: string | null;
  latest_goal?: string | null;
  latest_updated_at?: string | null;
};

export type WorkflowRun = {
  id: string;
  owner: { kind: string; id: string };
  conversation_id: string;
  lane_id?: string | null;
  trigger: string;
  stage: string;
  goal: string;
  created_at: string;
  updated_at: string;
};

export type WorkflowTask = {
  id: string;
  run_id: string;
  conversation_id: string;
  lane_id?: string | null;
  task_kind: string;
  title: string;
  assigned_agent_id: string;
  status: string;
  created_at: string;
  updated_at: string;
};

export type WorkflowArtifact = {
  id: string;
  owner: { kind: string; id: string };
  run_id: string;
  conversation_id?: string | null;
  lane_id?: string | null;
  kind: string;
  relative_path: string;
  created_at: string;
};

export type WorkflowHandoff = {
  id: string;
  from_lane_id: string;
  to_lane_id: string;
  from_agent_id?: string | null;
  to_agent_id?: string | null;
  summary: string;
  instructions: string;
  status: string;
  created_at: string;
};

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
  run: WorkflowRun;
  tasks: WorkflowTask[];
  artifacts: WorkflowArtifact[];
  handoffs: WorkflowHandoff[];
  stage_events: WorkflowStageEvent[];
  gate_verdicts: WorkflowGateVerdict[];
  decisions?: WorkflowDecisionSnapshot[];
};

type ExtensionRpcEnvelope<T> = {
  ok: boolean;
  data: T;
  error?: { code: string; message: string } | null;
};

async function callWorkflowRpc<T>(
  method: string,
  params?: Record<string, unknown>,
) {
  const response = await fetchJson<ExtensionRpcEnvelope<T>>(
    `/api/extensions/workflow/rpc/${method}`,
    {
      method: "POST",
      body: JSON.stringify({
        params: params ?? {},
      }),
    },
  );
  if (!response.ok) {
    throw new Error(response.error?.message ?? "workflow rpc failed");
  }
  return response.data;
}

export async function getWorkflowWorkspaceSummary() {
  return callWorkflowRpc<WorkflowWorkspaceSummary>("workspace");
}

export async function listWorkflowRuns(params?: {
  conversation_id?: string;
  stage?: string;
  trigger?: string;
  q?: string;
  limit?: number;
}) {
  return callWorkflowRpc<WorkflowRun[]>("workflow/runs/list-by-conversation", params);
}

export async function getWorkflowRunDetail(runId: string) {
  return callWorkflowRpc<WorkflowRunDetail>("workflow/runs/get", { run_id: runId });
}
