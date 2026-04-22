import { fetchJson } from "./core";
import type {
  AppConfig,
  BootstrapSetupResponse,
  BootstrapState,
  RuntimeProfile,
  ServerConfig,
  UiMessagesResponse,
  UiPreferenceRecord,
  UiRuntime,
} from "./types";

export async function fetchBootstrapStatus() {
  return fetchJson<BootstrapState>("/api/v1/bootstrap/status");
}

export async function bootstrapSetup(payload: {
  display_name?: string;
  locale?: string;
  time_zone?: string;
  default_space_id?: string;
  theme_id?: string;
  date_style?: string;
  density?: string;
  motion?: string;
}) {
  return fetchJson<BootstrapSetupResponse>("/api/v1/bootstrap/setup", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function fetchUiRuntime() {
  return fetchJson<UiRuntime>("/api/v1/ui/runtime");
}

export async function fetchUiMessages(locale: string, namespaces: string[] = []) {
  const params = new URLSearchParams({ locale });
  if (namespaces.length > 0) {
    params.set("namespaces", namespaces.join(","));
  }
  return fetchJson<UiMessagesResponse>(`/api/v1/ui/messages?${params.toString()}`);
}

export async function fetchRuntimeProfile() {
  return fetchJson<RuntimeProfile | null>("/api/v1/runtime/profile");
}

export async function saveRuntimeProfile(payload: {
  display_name?: string | null;
  locale?: string | null;
  time_zone?: string | null;
  default_space_id?: string | null;
}) {
  return fetchJson<RuntimeProfile>("/api/v1/runtime/profile", {
    method: "PUT",
    body: JSON.stringify(payload),
  });
}

export async function fetchRuntimePreferences() {
  return fetchJson<UiPreferenceRecord | null>("/api/v1/runtime/preferences");
}

export async function saveInstanceUiPreferences(payload: {
  locale?: string | null;
  theme_id?: string | null;
  time_zone?: string | null;
  date_style?: string | null;
  density?: string | null;
  motion?: string | null;
}) {
  return fetchJson<UiPreferenceRecord>("/api/v1/runtime/preferences", {
    method: "PUT",
    body: JSON.stringify(payload),
  });
}

export async function fetchAppConfig() {
  return fetchJson<AppConfig>("/api/v1/runtime/app-config");
}

export async function saveAppConfig(payload: AppConfig) {
  return fetchJson<AppConfig>("/api/v1/runtime/app-config", {
    method: "PUT",
    body: JSON.stringify(payload),
  });
}

export async function fetchServerConfig() {
  return fetchJson<ServerConfig>("/api/v1/runtime/server-config");
}

export async function saveServerConfig(payload: ServerConfig) {
  return fetchJson<ServerConfig>("/api/v1/runtime/server-config", {
    method: "PUT",
    body: JSON.stringify(payload),
  });
}

