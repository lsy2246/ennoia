import { fetchJson } from "./core";
import type { ConfigChangeRecord, RuntimeConfigEntry, SystemConfig } from "./types";

export async function listConfig() {
  return fetchJson<RuntimeConfigEntry[]>("/api/v1/runtime/config");
}

export async function getConfig(key: string) {
  return fetchJson<RuntimeConfigEntry>(`/api/v1/runtime/config/${key}`);
}

export async function putConfig(key: string, payload: unknown, updatedBy?: string) {
  return fetchJson<RuntimeConfigEntry>(`/api/v1/runtime/config/${key}`, {
    method: "PUT",
    body: JSON.stringify({ payload, updated_by: updatedBy }),
  });
}

export async function getConfigHistory(key: string) {
  return fetchJson<ConfigChangeRecord[]>(`/api/v1/runtime/config/${key}/history`);
}

export async function getConfigSnapshot() {
  return fetchJson<SystemConfig>("/api/v1/runtime/config/snapshot");
}
