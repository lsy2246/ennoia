import { dispatchAction } from "./actions";
import { apiUrl, fetchJson } from "./core";
import type {
  ConversationBranch,
  ConversationLane,
  ConversationMessage,
  ConversationMessageAppendResponse,
  ConversationSummary,
  ConversationDetail,
  ConversationStreamSnapshot,
  OperationRecord,
  ExecutionRun,
  ExecutionStep,
  ExtensionRecordEntry,
  PermissionApprovalRecord,
  RunOutput,
} from "./types";
import { listConversationExtensionRecords } from "./extension-runtime";
import type { FetchJsonInit } from "./core";

type ConversationDetailPayload = {
  conversation: ConversationSummary;
  lanes?: ConversationLane[];
  branches?: ConversationBranch[];
  messages?: ConversationMessage[];
  records?: ExtensionRecordEntry[];
  operations?: OperationRecord[];
  runs?: ExecutionRun[];
  tasks?: ExecutionStep[];
  outputs?: RunOutput[];
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function isConversationSummary(value: unknown): value is ConversationSummary {
  return isRecord(value)
    && typeof value.id === "string"
    && (value.topology === "direct" || value.topology === "group")
    && typeof value.title === "string";
}

function normalizeConversationDetailPayload(payload: unknown): ConversationDetailPayload {
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
    messages: Array.isArray(payload.messages) ? payload.messages as ConversationMessage[] : undefined,
    records: Array.isArray(payload.records) ? payload.records as ExtensionRecordEntry[] : undefined,
    operations: Array.isArray(payload.operations) ? payload.operations as OperationRecord[] : undefined,
    runs: Array.isArray(payload.runs) ? payload.runs as ExecutionRun[] : undefined,
    tasks: Array.isArray(payload.tasks) ? payload.tasks as ExecutionStep[] : undefined,
    outputs: Array.isArray(payload.outputs) ? payload.outputs as RunOutput[] : undefined,
  };
}

async function listConversationRuns(conversationId: string, init?: FetchJsonInit) {
  try {
    return await dispatchAction<ExecutionRun[]>("run.list", {
      conversation_id: conversationId,
      limit: 24,
    }, init);
  } catch {
    return [];
  }
}

async function listConversationOperations(conversationId: string, init?: FetchJsonInit) {
  try {
    return await fetchJson<OperationRecord[]>(
      apiUrl(`/api/operations?conversation_id=${encodeURIComponent(conversationId)}&limit=240`),
      init,
    );
  } catch {
    return [];
  }
}

async function hydrateConversationDetail(
  conversationId: string,
  payload: unknown,
  init?: FetchJsonInit,
): Promise<ConversationDetail> {
  const normalized = normalizeConversationDetailPayload(payload);
  const [lanes, branches, messages, records, operations, runs] = await Promise.all([
    normalized.lanes
      ? Promise.resolve(normalized.lanes)
      : dispatchAction<ConversationLane[]>("lane.list", { conversation_id: conversationId }, init),
    normalized.branches
      ? Promise.resolve(normalized.branches)
      : dispatchAction<ConversationBranch[]>("branch.list", { conversation_id: conversationId }, init),
    normalized.messages
      ? Promise.resolve(normalized.messages)
      : dispatchAction<ConversationMessage[]>("message.list", { conversation_id: conversationId }, init),
    normalized.records
      ? Promise.resolve(normalized.records)
      : listConversationExtensionRecords(conversationId, 120, init),
    normalized.operations
      ? Promise.resolve(normalized.operations)
      : listConversationOperations(conversationId, init),
    normalized.runs ? Promise.resolve(normalized.runs) : listConversationRuns(conversationId, init),
  ]);

  return {
    conversation: normalized.conversation,
    lanes,
    branches,
    messages,
    records,
    operations,
    runs,
    tasks: normalized.tasks ?? [],
    outputs: normalized.outputs ?? [],
  };
}

export async function listConversations() {
  return dispatchAction<ConversationSummary[]>("conversation.list");
}

export async function createConversation(payload: {
  topology: "direct" | "group";
  title?: string;
  agent_ids: string[];
  lane_name?: string;
  lane_type?: string;
  lane_goal?: string;
}) {
  return dispatchAction<{ conversation: ConversationSummary; default_lane: ConversationLane }>(
    "conversation.create",
    payload,
  );
}

export async function deleteConversation(conversationId: string) {
  await dispatchAction("conversation.delete", { conversation_id: conversationId });
}

export async function getConversation(
  conversationId: string,
  init?: FetchJsonInit,
): Promise<ConversationDetail> {
  const detail = await dispatchAction<unknown>("conversation.get", {
    conversation_id: conversationId,
  }, init);
  return hydrateConversationDetail(conversationId, detail, init);
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
    parent_message_id?: string;
    fork_from_message_id?: string;
    rewrite_from_message_id?: string;
    branch_name?: string;
  },
) {
  return dispatchAction<ConversationMessageAppendResponse>("message.append", {
    conversation_id: conversationId,
    message: {
      ...payload,
      role: "operator",
      sender: "operator",
    },
  });
}

export async function listConversationLanes(conversationId: string) {
  return dispatchAction<ConversationLane[]>("lane.list", {
    conversation_id: conversationId,
  });
}

export async function listConversationBranches(conversationId: string) {
  return dispatchAction<ConversationBranch[]>("branch.list", {
    conversation_id: conversationId,
  });
}

export async function createConversationBranch(
  conversationId: string,
  payload: {
    from_branch_id?: string;
    source_message_id?: string;
    name?: string;
    mode?: "fork" | "rewrite";
    activate?: boolean;
  },
) {
  return dispatchAction<ConversationBranch>("branch.create", {
    conversation_id: conversationId,
    ...payload,
  });
}

export async function switchConversationBranch(conversationId: string, branchId: string) {
  await dispatchAction("branch.switch", {
    conversation_id: conversationId,
    branch_id: branchId,
  });
  return getConversation(conversationId);
}

export async function updateConversationBranch(
  conversationId: string,
  branchId: string,
  payload: {
    name: string;
  },
) {
  return dispatchAction<ConversationBranch>("branch.update", {
    conversation_id: conversationId,
    branch_id: branchId,
    ...payload,
  });
}

export async function deleteConversationBranch(
  conversationId: string,
  branchId: string,
  payload: {
    mode: "detach_children" | "delete_tree";
  },
) {
  await dispatchAction("branch.delete", {
    conversation_id: conversationId,
    branch_id: branchId,
    ...payload,
  });
  return getConversation(conversationId);
}

export function createConversationStream(conversationId: string) {
  return new EventSource(apiUrl(`/api/conversations/${encodeURIComponent(conversationId)}/stream`));
}

export function createConversationsStream() {
  return new EventSource(apiUrl("/api/conversations/stream"));
}

export function parseConversationStreamPayload(value: string): ConversationStreamSnapshot {
  const parsed = JSON.parse(value) as {
    detail?: unknown;
    approvals?: unknown;
    operations?: unknown;
  };
  const detailValue = parsed.detail;
  const normalized = normalizeConversationDetailPayload(detailValue);
  const detailRecord = isRecord(detailValue) ? detailValue : null;

  return {
    detail: {
      conversation: normalized.conversation,
      lanes: normalized.lanes ?? [],
      branches: normalized.branches ?? [],
      messages: normalized.messages ?? [],
      records: Array.isArray(detailRecord?.records) ? detailRecord.records as ConversationDetail["records"] : [],
      operations: Array.isArray(parsed.operations)
        ? parsed.operations as ConversationDetail["operations"]
        : Array.isArray(detailRecord?.operations)
          ? detailRecord.operations as ConversationDetail["operations"]
          : [],
      runs: Array.isArray(detailRecord?.runs) ? detailRecord.runs as ConversationDetail["runs"] : [],
      tasks: Array.isArray(detailRecord?.tasks) ? detailRecord.tasks as ConversationDetail["tasks"] : [],
      outputs: Array.isArray(detailRecord?.outputs) ? detailRecord.outputs as ConversationDetail["outputs"] : [],
    },
    approvals: Array.isArray(parsed.approvals)
      ? parsed.approvals as PermissionApprovalRecord[]
      : [],
    operations: Array.isArray(parsed.operations)
      ? parsed.operations as OperationRecord[]
      : Array.isArray(detailRecord?.operations)
        ? detailRecord.operations as OperationRecord[]
        : [],
  };
}
