import { fetchJson } from "./core";
import type { ChatLane, ChatMessage, ChatSendResponse, ChatThread, ChatThreadDetail } from "./types";

const SESSION_API = "/api/ext/session";

export async function listChats() {
  return fetchJson<ChatThread[]>(`${SESSION_API}/conversations`);
}

export async function createChat(payload: {
  topology: "direct" | "group";
  title?: string;
  agent_ids: string[];
  lane_name?: string;
  lane_type?: string;
  lane_goal?: string;
}) {
  return fetchJson<{ conversation: ChatThread; default_lane: ChatLane }>(`${SESSION_API}/conversations`, {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function deleteChat(chatId: string) {
  return fetchJson<void>(`${SESSION_API}/conversations/${chatId}`, { method: "DELETE" });
}

export async function getChat(chatId: string): Promise<ChatThreadDetail> {
  const [detail, messages] = await Promise.all([
    fetchJson<{ conversation: ChatThread; lanes: ChatLane[] }>(`${SESSION_API}/conversations/${chatId}`),
    fetchJson<ChatMessage[]>(`${SESSION_API}/conversations/${chatId}/messages`),
  ]);
  return {
    conversation: detail.conversation,
    lanes: detail.lanes,
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
  },
) {
  return fetchJson<ChatSendResponse>(`${SESSION_API}/conversations/${chatId}/messages`, {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

