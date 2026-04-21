import { fetchJson } from "./core";
import type { MemoryRecord, RecallResult, ReviewReceipt } from "./types";

export async function listMemories() {
  return fetchJson<MemoryRecord[]>("/api/v1/memories");
}

export async function recallMemories(payload: {
  owner_kind: string;
  owner_id: string;
  query_text?: string;
  namespace_prefix?: string;
  memory_kind?: string;
  mode?: string;
  limit?: number;
  conversation_id?: string;
  run_id?: string;
}) {
  return fetchJson<RecallResult>("/api/v1/memories/recall", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function reviewMemory(payload: {
  target_memory_id: string;
  reviewer: string;
  action: string;
  notes?: string;
}) {
  return fetchJson<ReviewReceipt>("/api/v1/memories/review", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

