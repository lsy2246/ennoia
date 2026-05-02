import { apiUrl, fetchJson } from "./core";
import type {
  ConversationBranch,
  ConversationCheckpoint,
  ConversationLane,
  ConversationMessage,
  ConversationMessageAppendResponse,
  ConversationSummary,
  ConversationDetail,
  ConversationStreamSnapshot,
  ExecutionRun,
  ExecutionStep,
  PermissionApprovalRecord,
  RunOutput,
} from "./types";

const CONVERSATIONS_API = "/api/conversations";

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function isConversationSummary(value: unknown): value is ConversationSummary {
  return isRecord(value)
    && typeof value.id === "string"
    && (value.topology === "direct" || value.topology === "group")
    && typeof value.title === "string";
}

function normalizeConversationDetailPayload(payload: unknown): {
  conversation: ConversationSummary;
  lanes?: ConversationLane[];
  branches?: ConversationBranch[];
  checkpoints?: ConversationCheckpoint[];
  messages?: ConversationMessage[];
  runs?: ExecutionRun[];
  tasks?: ExecutionStep[];
  outputs?: RunOutput[];
} {
  if (isConversationSummary(payload)) {
    return { conversation: payload };
  }

  if (!isRecord(payload) || !isConversationSummary(payload.conversation)) {
    throw new Error("invalid conversation detail payload");
  }

  return {
    conversation: payload.conversation,
    lanes: Array.isArray(payload.lanes) ? payload.lanes as ConversationLane[] : undefined,
    branches: Array.isArray(payload.branches) ? payload.branches as ConversationBranch[] : undefined,
    checkpoints: Array.isArray(payload.checkpoints) ? payload.checkpoints as ConversationCheckpoint[] : undefined,
    messages: Array.isArray(payload.messages) ? payload.messages as ConversationMessage[] : undefined,
    runs: Array.isArray(payload.runs) ? payload.runs as ExecutionRun[] : undefined,
    tasks: Array.isArray(payload.tasks) ? payload.tasks as ExecutionStep[] : undefined,
    outputs: Array.isArray(payload.outputs) ? payload.outputs as RunOutput[] : undefined,
  };
}

export async function listConversations() {
  return fetchJson<ConversationSummary[]>(CONVERSATIONS_API);
}

export async function createConversation(payload: {
  topology: "direct" | "group";
  title?: string;
  agent_ids: string[];
  lane_name?: string;
  lane_type?: string;
  lane_goal?: string;
}) {
  return fetchJson<{ conversation: ConversationSummary; default_lane: ConversationLane }>(CONVERSATIONS_API, {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function deleteConversation(conversationId: string) {
  return fetchJson<void>(`${CONVERSATIONS_API}/${conversationId}`, { method: "DELETE" });
}

export async function getConversation(conversationId: string): Promise<ConversationDetail> {
  const detail = await fetchJson<unknown>(`${CONVERSATIONS_API}/${conversationId}`);
  const normalized = normalizeConversationDetailPayload(detail);
  const lanes = normalized.lanes ?? await fetchJson<ConversationLane[]>(`${CONVERSATIONS_API}/${conversationId}/lanes`);
  const branches = normalized.branches ?? await fetchJson<ConversationBranch[]>(`${CONVERSATIONS_API}/${conversationId}/branches`);
  const checkpoints = normalized.checkpoints ?? await fetchJson<ConversationCheckpoint[]>(`${CONVERSATIONS_API}/${conversationId}/checkpoints`);
  const messages = normalized.messages
    ?? await fetchJson<ConversationMessage[]>(`${CONVERSATIONS_API}/${conversationId}/messages`);
  return {
    conversation: normalized.conversation,
    lanes,
    branches,
    checkpoints,
    messages,
    runs: normalized.runs ?? [],
    tasks: normalized.tasks ?? [],
    outputs: normalized.outputs ?? [],
  };
}

export async function appendConversationMessage(
  conversationId: string,
  payload: {
    lane_id?: string;
    branch_id?: string;
    body: string;
    goal?: string;
    addressed_agents?: string[];
    mentions?: string[];
    fork_from_message_id?: string;
    rewrite_from_message_id?: string;
    reset_context?: boolean;
    branch_name?: string;
  },
) {
  return fetchJson<ConversationMessageAppendResponse>(`${CONVERSATIONS_API}/${conversationId}/messages`, {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function listConversationLanes(conversationId: string) {
  return fetchJson<ConversationLane[]>(`${CONVERSATIONS_API}/${conversationId}/lanes`);
}

export async function listConversationBranches(conversationId: string) {
  return fetchJson<ConversationBranch[]>(`${CONVERSATIONS_API}/${conversationId}/branches`);
}

export async function createConversationBranch(
  conversationId: string,
  payload: {
    from_branch_id?: string;
    source_message_id?: string;
    source_checkpoint_id?: string;
    name?: string;
    mode?: "fork" | "rewrite" | "reset";
    activate?: boolean;
  },
) {
  return fetchJson<ConversationBranch>(`${CONVERSATIONS_API}/${conversationId}/branches`, {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function switchConversationBranch(conversationId: string, branchId: string) {
  const detail = await fetchJson<unknown>(`${CONVERSATIONS_API}/${conversationId}/branches/${branchId}/switch`, {
    method: "POST",
  });
  const normalized = normalizeConversationDetailPayload(detail);
  return {
    conversation: normalized.conversation,
    lanes: normalized.lanes ?? [],
    branches: normalized.branches ?? [],
    checkpoints: normalized.checkpoints ?? [],
    messages: normalized.messages ?? [],
    runs: normalized.runs ?? [],
    tasks: normalized.tasks ?? [],
    outputs: normalized.outputs ?? [],
  } satisfies ConversationDetail;
}

export async function createConversationCheckpoint(
  conversationId: string,
  payload: {
    branch_id?: string;
    message_id?: string;
    kind?: string;
    label?: string;
  },
) {
  return fetchJson<ConversationCheckpoint>(`${CONVERSATIONS_API}/${conversationId}/checkpoints`, {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export function createConversationStream(conversationId: string) {
  return new EventSource(
    apiUrl(`${CONVERSATIONS_API}/${encodeURIComponent(conversationId)}/stream`),
  );
}

export function parseConversationStreamPayload(value: string): ConversationStreamSnapshot {
  const parsed = JSON.parse(value) as {
    detail?: unknown;
    approvals?: unknown;
  };
  const detailValue = parsed.detail;
  const normalized = normalizeConversationDetailPayload(detailValue);
  const detailRecord = isRecord(detailValue) ? detailValue : null;

  return {
    detail: {
      conversation: normalized.conversation,
      lanes: normalized.lanes ?? [],
      branches: normalized.branches ?? [],
      checkpoints: normalized.checkpoints ?? [],
      messages: normalized.messages ?? [],
      runs: Array.isArray(detailRecord?.runs) ? detailRecord.runs as ConversationDetail["runs"] : [],
      tasks: Array.isArray(detailRecord?.tasks) ? detailRecord.tasks as ConversationDetail["tasks"] : [],
      outputs: Array.isArray(detailRecord?.outputs) ? detailRecord.outputs as ConversationDetail["outputs"] : [],
    },
    approvals: Array.isArray(parsed.approvals)
      ? parsed.approvals as PermissionApprovalRecord[]
      : [],
  };
}
