import { apiUrl, fetchJson } from "./core";
import type {
  ChatBranch,
  ChatCheckpoint,
  ChatLane,
  ChatMessage,
  ChatSendResponse,
  ChatThread,
  ChatThreadDetail,
  ConversationStreamSnapshot,
  PermissionApprovalRecord,
} from "./types";

const CONVERSATIONS_API = "/api/conversations";

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function isChatThread(value: unknown): value is ChatThread {
  return isRecord(value)
    && typeof value.id === "string"
    && (value.topology === "direct" || value.topology === "group")
    && typeof value.title === "string";
}

function normalizeChatDetailPayload(payload: unknown): {
  conversation: ChatThread;
  lanes?: ChatLane[];
  branches?: ChatBranch[];
  checkpoints?: ChatCheckpoint[];
  messages?: ChatMessage[];
} {
  if (isChatThread(payload)) {
    return { conversation: payload };
  }

  if (!isRecord(payload) || !isChatThread(payload.conversation)) {
    throw new Error("invalid conversation detail payload");
  }

  return {
    conversation: payload.conversation,
    lanes: Array.isArray(payload.lanes) ? payload.lanes as ChatLane[] : undefined,
    branches: Array.isArray(payload.branches) ? payload.branches as ChatBranch[] : undefined,
    checkpoints: Array.isArray(payload.checkpoints) ? payload.checkpoints as ChatCheckpoint[] : undefined,
    messages: Array.isArray(payload.messages) ? payload.messages as ChatMessage[] : undefined,
  };
}

export async function listChats() {
  return fetchJson<ChatThread[]>(CONVERSATIONS_API);
}

export async function createChat(payload: {
  topology: "direct" | "group";
  title?: string;
  agent_ids: string[];
  lane_name?: string;
  lane_type?: string;
  lane_goal?: string;
}) {
  return fetchJson<{ conversation: ChatThread; default_lane: ChatLane }>(CONVERSATIONS_API, {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function deleteChat(chatId: string) {
  return fetchJson<void>(`${CONVERSATIONS_API}/${chatId}`, { method: "DELETE" });
}

export async function getChat(chatId: string): Promise<ChatThreadDetail> {
  const detail = await fetchJson<unknown>(`${CONVERSATIONS_API}/${chatId}`);
  const normalized = normalizeChatDetailPayload(detail);
  const lanes = normalized.lanes ?? await fetchJson<ChatLane[]>(`${CONVERSATIONS_API}/${chatId}/lanes`);
  const branches = normalized.branches ?? await fetchJson<ChatBranch[]>(`${CONVERSATIONS_API}/${chatId}/branches`);
  const checkpoints = normalized.checkpoints ?? await fetchJson<ChatCheckpoint[]>(`${CONVERSATIONS_API}/${chatId}/checkpoints`);
  const messages = normalized.messages
    ?? await fetchJson<ChatMessage[]>(`${CONVERSATIONS_API}/${chatId}/messages`);
  return {
    conversation: normalized.conversation,
    lanes,
    branches,
    checkpoints,
    messages,
    runs: [],
    tasks: [],
    outputs: [],
  };
}

export async function sendChatMessage(
  chatId: string,
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
  return fetchJson<ChatSendResponse>(`${CONVERSATIONS_API}/${chatId}/messages`, {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function listChatLanes(chatId: string) {
  return fetchJson<ChatLane[]>(`${CONVERSATIONS_API}/${chatId}/lanes`);
}

export async function listChatBranches(chatId: string) {
  return fetchJson<ChatBranch[]>(`${CONVERSATIONS_API}/${chatId}/branches`);
}

export async function createChatBranch(
  chatId: string,
  payload: {
    from_branch_id?: string;
    source_message_id?: string;
    source_checkpoint_id?: string;
    name?: string;
    mode?: "fork" | "rewrite" | "reset";
    activate?: boolean;
  },
) {
  return fetchJson<ChatBranch>(`${CONVERSATIONS_API}/${chatId}/branches`, {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function switchChatBranch(chatId: string, branchId: string) {
  const detail = await fetchJson<unknown>(`${CONVERSATIONS_API}/${chatId}/branches/${branchId}/switch`, {
    method: "POST",
  });
  const normalized = normalizeChatDetailPayload(detail);
  return {
    conversation: normalized.conversation,
    lanes: normalized.lanes ?? [],
    branches: normalized.branches ?? [],
    checkpoints: normalized.checkpoints ?? [],
    messages: normalized.messages ?? [],
    runs: [],
    tasks: [],
    outputs: [],
  } satisfies ChatThreadDetail;
}

export async function createChatCheckpoint(
  chatId: string,
  payload: {
    branch_id?: string;
    message_id?: string;
    kind?: string;
    label?: string;
  },
) {
  return fetchJson<ChatCheckpoint>(`${CONVERSATIONS_API}/${chatId}/checkpoints`, {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export function createConversationStream(chatId: string) {
  return new EventSource(
    apiUrl(`${CONVERSATIONS_API}/${encodeURIComponent(chatId)}/stream`),
  );
}

export function parseConversationStreamPayload(value: string): ConversationStreamSnapshot {
  const parsed = JSON.parse(value) as {
    detail?: unknown;
    approvals?: unknown;
  };
  const detailValue = parsed.detail;
  const normalized = normalizeChatDetailPayload(detailValue);
  const detailRecord = isRecord(detailValue) ? detailValue : null;

  return {
    detail: {
      conversation: normalized.conversation,
      lanes: normalized.lanes ?? [],
      branches: normalized.branches ?? [],
      checkpoints: normalized.checkpoints ?? [],
      messages: normalized.messages ?? [],
      runs: Array.isArray(detailRecord?.runs) ? detailRecord.runs as ChatThreadDetail["runs"] : [],
      tasks: Array.isArray(detailRecord?.tasks) ? detailRecord.tasks as ChatThreadDetail["tasks"] : [],
      outputs: Array.isArray(detailRecord?.outputs) ? detailRecord.outputs as ChatThreadDetail["outputs"] : [],
    },
    approvals: Array.isArray(parsed.approvals)
      ? parsed.approvals as PermissionApprovalRecord[]
      : [],
  };
}
