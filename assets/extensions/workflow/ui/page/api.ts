import { apiUrl, fetchJson } from "@ennoia/api-client";

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

export type WorkflowPlanStep = {
  id: string;
  type: string;
  goal: string;
  tool?: string;
  allowed_tools?: string[];
  expected_outputs?: string[];
  pass_if?: string[];
  next_pass?: string;
  next_fail?: string;
  assigned_agent_id?: string;
};

export type WorkflowPlan = {
  schema_version: string;
  objective: string;
  intent: string;
  steps: WorkflowPlanStep[];
  tool_plan?: Array<Record<string, unknown>>;
  verify_contract?: Record<string, unknown> | null;
  delegation?: Record<string, unknown> | null;
  watchdog?: Record<string, unknown> | null;
  model_strategy?: Record<string, unknown> | null;
  meta?: {
    plan_status?: string;
    source?: string;
    auto_generated?: boolean;
    created_at?: string;
    updated_at?: string;
  };
};

export type WorkflowRunDetail = {
  run: WorkflowRun;
  plan?: WorkflowPlan | null;
  tasks: WorkflowTask[];
  artifacts: WorkflowArtifact[];
  handoffs: WorkflowHandoff[];
  stage_events: WorkflowStageEvent[];
  gate_verdicts: WorkflowGateVerdict[];
  decisions?: WorkflowDecisionSnapshot[];
};

export type WorkflowStreamSnapshot = {
  workspace: WorkflowWorkspaceSummary;
  runs: WorkflowRun[];
  detail?: WorkflowRunDetail | null;
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
  return callWorkflowRpc<WorkflowWorkspaceSummary>("workflow.workspace");
}

export async function listWorkflowRuns(params?: {
  conversation_id?: string;
  stage?: string;
  trigger?: string;
  q?: string;
  limit?: number;
}) {
  return callWorkflowRpc<WorkflowRun[]>("run.list", params);
}

export async function getWorkflowRunDetail(runId: string) {
  return callWorkflowRpc<WorkflowRunDetail>("run.get", { run_id: runId });
}

export function createWorkflowStream(query?: {
  conversation_id?: string;
  run_id?: string;
  stage?: string;
  q?: string;
  limit?: number;
}) {
  const params = new URLSearchParams();
  if (query?.conversation_id) {
    params.set("conversation_id", query.conversation_id);
  }
  if (query?.run_id) {
    params.set("run_id", query.run_id);
  }
  if (query?.stage) {
    params.set("stage", query.stage);
  }
  if (query?.q) {
    params.set("q", query.q);
  }
  if (typeof query?.limit === "number") {
    params.set("limit", String(query.limit));
  }
  const suffix = params.toString();
  return new EventSource(apiUrl(`/api/workflow/stream${suffix ? `?${suffix}` : ""}`));
}

export function parseWorkflowStreamPayload(value: string) {
  return JSON.parse(value) as WorkflowStreamSnapshot;
}
