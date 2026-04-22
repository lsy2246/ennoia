import { fetchJson } from "@ennoia/api-client";

export type MemorySource = {
  kind: string;
  reference: string;
};

export type MemoryRecord = {
  id: string;
  owner: { kind: string; id: string };
  namespace: string;
  memory_kind: string;
  stability: string;
  status: string;
  superseded_by?: string | null;
  title?: string | null;
  content: string;
  summary?: string | null;
  confidence: number;
  importance: number;
  valid_from?: string | null;
  valid_to?: string | null;
  sources: MemorySource[];
  tags: string[];
  entities: string[];
  created_at: string;
  updated_at: string;
};

export type RecallResult = {
  memories: MemoryRecord[];
  receipt_id: string;
  mode: string;
  total_chars: number;
};

export async function listMemoryRecords() {
  return fetchJson<MemoryRecord[]>("/api/ext/memory/memories");
}

export async function recallMemoryRecords(payload: {
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
  return fetchJson<RecallResult>("/api/ext/memory/memories/recall", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function reviewMemoryRecord(payload: {
  target_memory_id: string;
  reviewer: string;
  action: string;
  notes?: string;
}) {
  return fetchJson("/api/ext/memory/memories/review", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}
