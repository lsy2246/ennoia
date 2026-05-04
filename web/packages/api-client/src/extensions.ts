import { apiUrl, fetchJson, toQueryString } from "./core";
import type { ExtensionDiagnostic } from "@ennoia/ui-sdk";
import type {
  ExtensionDetail,
  ExtensionRuntimeEvent,
  ExtensionRuntimeState,
  ExtensionSettingsResponse,
} from "./types";

export async function listExtensions() {
  return fetchJson<ExtensionRuntimeState[]>("/api/extensions");
}

export async function getExtension(extensionId: string) {
  return fetchJson<ExtensionDetail>(`/api/extensions/${extensionId}`);
}

export async function getExtensionSettings(extensionId: string) {
  return fetchJson<ExtensionSettingsResponse>(`/api/extensions/${extensionId}/settings`);
}

export async function saveExtensionSettings(
  extensionId: string,
  values: Record<string, string | number | boolean>,
) {
  return fetchJson<ExtensionSettingsResponse>(`/api/extensions/${extensionId}/settings`, {
    method: "PUT",
    body: JSON.stringify({ values }),
  });
}

export async function getExtensionDiagnostics(extensionId: string) {
  return fetchJson<ExtensionDiagnostic[]>(`/api/extensions/${extensionId}/diagnostics`);
}

export async function setExtensionEnabled(extensionId: string, enabled: boolean) {
  return fetchJson<ExtensionRuntimeState>(`/api/extensions/${extensionId}/enabled`, {
    method: "PUT",
    body: JSON.stringify({ enabled }),
  });
}

export async function reloadExtension(extensionId: string) {
  return fetchJson<ExtensionDetail>(`/api/extensions/${extensionId}/reload`, {
    method: "POST",
    body: JSON.stringify({}),
  });
}

export async function restartExtension(extensionId: string) {
  return fetchJson<ExtensionDetail>(`/api/extensions/${extensionId}/restart`, {
    method: "POST",
    body: JSON.stringify({}),
  });
}

export async function getExtensionLogs(extensionId: string) {
  const response = await fetch(apiUrl(`/api/extensions/${extensionId}/logs`));
  return response.text();
}

export async function listExtensionEvents(limit = 100) {
  return fetchJson<ExtensionRuntimeEvent[]>(
    `/api/extensions/events${toQueryString({ limit })}`,
  );
}

export function createExtensionEventsStream() {
  return new EventSource(apiUrl("/api/extensions/events/stream"));
}

export function getExtensionUiModuleUrl(extensionId: string) {
  return apiUrl(`/api/extensions/${encodeURIComponent(extensionId)}/ui/module`);
}

export function getExtensionUiAssetUrl(extensionId: string, assetPath: string) {
  return apiUrl(`/api/extensions/${encodeURIComponent(extensionId)}/ui/assets/${assetPath}`);
}

export function getExtensionThemeStylesheetUrl(extensionId: string, themeId: string) {
  return apiUrl(`/api/extensions/${extensionId}/themes/${encodeURIComponent(themeId)}/stylesheet`);
}

type ExtensionRpcEnvelope<T> = {
  ok: boolean;
  data: T;
  error?: { code: string; message: string } | null;
};

export async function callExtensionRpc<T>(
  extensionId: string,
  method: string,
  params?: Record<string, unknown> | null,
) {
  const response = await fetchJson<ExtensionRpcEnvelope<T>>(
    `/api/extensions/${encodeURIComponent(extensionId)}/rpc/${method}`,
    {
      method: "POST",
      body: JSON.stringify({
        params: params ?? {},
      }),
    },
  );
  if (!response.ok) {
    throw new Error(response.error?.message ?? "extension rpc failed");
  }
  return response.data;
}

export async function invokeProviderMethod<T>(
  providerKind: string,
  method: string,
  params?: Record<string, unknown> | null,
) {
  return fetchJson<T>(
    `/api/extensions/providers/${encodeURIComponent(providerKind)}/${method}`,
    {
      method: "POST",
      body: JSON.stringify({
        params: params ?? {},
      }),
    },
  );
}

