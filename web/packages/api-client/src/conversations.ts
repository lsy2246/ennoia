import { fetchJson } from "./core";
import type { ChatLane, ChatMessage, ChatSendResponse, ChatThread, ChatThreadDetail, ExecutionRun, ExecutionStep, RunOutput } from "./types";

export async function listChats() {
  return fetchJson<ChatThread[]>("/api/v1/conversations");
}

export async function createChat(payload: {
  topology: "direct" | "group";
  title?: string;
  agent_ids: string[];
  lane_name?: string;
  lane_type?: string;
  lane_goal?: string;
}) {
  return fetchJson<{ conversation: ChatThread; default_lane: ChatLane }>("/api/v1/conversations", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function deleteChat(chatId: string) {
  return fetchJson<void>(`/api/v1/conversations/${chatId}`, { method: "DELETE" });
}

export async function getChat(chatId: string): Promise<ChatThreadDetail> {
  const [detail, messages, runs] = await Promise.all([
    fetchJson<{ conversation: ChatThread; lanes: ChatLane[] }>(`/api/v1/conversations/${chatId}`),
    fetchJson<ChatMessage[]>(`/api/v1/conversations/${chatId}/messages`),
    fetchJson<ExecutionRun[]>(`/api/v1/conversations/${chatId}/runs`),
  ]);
  const taskBuckets = await Promise.all(
    runs.map((run) => fetchJson<ExecutionStep[]>(`/api/v1/runs/${run.id}/tasks`)),
  );
  const outputBuckets = await Promise.all(
    runs.map((run) => fetchJson<RunOutput[]>(`/api/v1/runs/${run.id}/artifacts`)),
  );
  return {
    conversation: detail.conversation,
    lanes: detail.lanes,
    messages,
    runs,
    tasks: taskBuckets.flat(),
    outputs: outputBuckets.flat(),
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
  return fetchJson<ChatSendResponse>(`/api/v1/conversations/${chatId}/messages`, {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

