import { fetchJson } from "./core";
import type { ChatLane, ChatMessage, ChatSendResponse, ChatThread, ChatThreadDetail } from "./types";

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
  const [detail, messages] = await Promise.all([
    fetchJson<unknown>(`${CONVERSATIONS_API}/${chatId}`),
    fetchJson<ChatMessage[]>(`${CONVERSATIONS_API}/${chatId}/messages`),
  ]);
  const normalized = normalizeChatDetailPayload(detail);
  const lanes = normalized.lanes ?? await fetchJson<ChatLane[]>(`${CONVERSATIONS_API}/${chatId}/lanes`);
  return {
    conversation: normalized.conversation,
    lanes,
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
    body: string;
    goal?: string;
    addressed_agents?: string[];
    mentions?: string[];
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
