import { apiUrl, fetchJson, toQueryString } from "./core";
import type { ExtensionDiagnostic } from "@ennoia/ui-sdk";
import type { ExtensionDetail, ExtensionRuntimeEvent, ExtensionRuntimeState } from "./types";

export async function listExtensions() {
  return fetchJson<ExtensionRuntimeState[]>("/api/v1/extensions");
}

export async function getExtension(extensionId: string) {
  return fetchJson<ExtensionDetail>(`/api/v1/extensions/${extensionId}`);
}

export async function getExtensionDiagnostics(extensionId: string) {
  return fetchJson<ExtensionDiagnostic[]>(`/api/v1/extensions/${extensionId}/diagnostics`);
}

export async function setExtensionEnabled(extensionId: string, enabled: boolean) {
  return fetchJson<ExtensionRuntimeState>(`/api/v1/extensions/${extensionId}/enabled`, {
    method: "PUT",
    body: JSON.stringify({ enabled }),
  });
}

export async function reloadExtension(extensionId: string) {
  return fetchJson<ExtensionDetail>(`/api/v1/extensions/${extensionId}/reload`, {
    method: "POST",
    body: JSON.stringify({}),
  });
}

export async function restartExtension(extensionId: string) {
  return fetchJson<ExtensionDetail>(`/api/v1/extensions/${extensionId}/restart`, {
    method: "POST",
    body: JSON.stringify({}),
  });
}

export async function getExtensionLogs(extensionId: string) {
  const response = await fetch(apiUrl(`/api/v1/extensions/${extensionId}/logs`));
  return response.text();
}

export async function listExtensionEvents(limit = 100) {
  return fetchJson<ExtensionRuntimeEvent[]>(
    `/api/v1/extensions/events${toQueryString({ limit })}`,
  );
}

export function getExtensionFrontendModuleUrl(extensionId: string) {
  return apiUrl(`/api/v1/extensions/${extensionId}/frontend/module`);
}

export function getExtensionThemeStylesheetUrl(extensionId: string, themeId: string) {
  return apiUrl(`/api/v1/extensions/${extensionId}/themes/${encodeURIComponent(themeId)}/stylesheet`);
}

