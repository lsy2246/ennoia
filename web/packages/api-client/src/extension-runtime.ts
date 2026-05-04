import { fetchJson, toQueryString, type FetchJsonInit } from "./core";
import type { ExtensionRecordEntry, ExtensionStateEntry } from "./types";

export async function getExtensionState(params: {
  extension_id: string;
  namespace: string;
  scope_type: string;
  scope_id: string;
  key: string;
}) {
  return fetchJson<ExtensionStateEntry>(`/api/extensions/state/item${toQueryString(params)}`);
}

export async function listExtensionState(params: {
  extension_id?: string;
  namespace?: string;
  scope_type?: string;
  scope_id?: string;
  key?: string;
  limit?: number;
}) {
  return fetchJson<ExtensionStateEntry[]>(`/api/extensions/state${toQueryString(params)}`);
}

export async function putExtensionState(payload: {
  extension_id: string;
  namespace: string;
  scope_type: string;
  scope_id: string;
  key: string;
  value: unknown;
  expires_at?: string | null;
}) {
  return fetchJson<ExtensionStateEntry>("/api/extensions/state", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function deleteExtensionState(params: {
  extension_id: string;
  namespace: string;
  scope_type: string;
  scope_id: string;
  key: string;
}) {
  return fetchJson<{ deleted: boolean }>(`/api/extensions/state${toQueryString(params)}`, {
    method: "DELETE",
  });
}

export async function listExtensionRecords(params: {
  extension_id?: string;
  namespace?: string;
  scope_type?: string;
  scope_id?: string;
  kind?: string;
  related_message_id?: string;
  open_only?: boolean;
  limit?: number;
}, init?: FetchJsonInit) {
  return fetchJson<ExtensionRecordEntry[]>(`/api/extensions/records${toQueryString(params)}`, init);
}

export async function getExtensionRecord(recordId: string) {
  return fetchJson<ExtensionRecordEntry>(`/api/extensions/records/${encodeURIComponent(recordId)}`);
}

export async function appendExtensionRecord(payload: {
  extension_id: string;
  namespace: string;
  scope_type: string;
  scope_id: string;
  kind: string;
  status?: string | null;
  title?: string | null;
  summary?: string | null;
  payload?: unknown;
  related_message_id?: string | null;
  parent_id?: string | null;
}) {
  return fetchJson<ExtensionRecordEntry>("/api/extensions/records", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function updateExtensionRecord(payload: {
  id: string;
  status?: string | null;
  title?: string | null;
  summary?: string | null;
  payload?: unknown;
  related_message_id?: string | null;
  parent_id?: string | null;
}) {
  return fetchJson<ExtensionRecordEntry>("/api/extensions/records", {
    method: "PUT",
    body: JSON.stringify(payload),
  });
}

export async function closeExtensionRecord(recordId: string) {
  return fetchJson<ExtensionRecordEntry>(`/api/extensions/records/${encodeURIComponent(recordId)}/close`, {
    method: "POST",
  });
}

export async function listConversationExtensionRecords(
  conversationId: string,
  limit = 120,
  init?: FetchJsonInit,
) {
  return listExtensionRecords({
    scope_type: "conversation",
    scope_id: conversationId,
    limit,
  }, init);
}
