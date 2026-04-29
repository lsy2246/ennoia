import { fetchJson } from "./core";
import type {
  BootstrapSetupResponse,
  BootstrapState,
  RuntimeProfile,
  ServerConfig,
  UiMessagesResponse,
  UiPreferenceRecord,
  UiRuntime,
} from "./types";

export async function fetchBootstrapStatus() {
  return fetchJson<BootstrapState>("/api/bootstrap/status");
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
  return fetchJson<BootstrapSetupResponse>("/api/bootstrap/setup", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function fetchUiRuntime() {
  return fetchJson<UiRuntime>("/api/ui/runtime");
}

export async function fetchUiMessages(locale: string, namespaces: string[] = []) {
  const params = new URLSearchParams({ locale });
  if (namespaces.length > 0) {
    params.set("namespaces", namespaces.join(","));
  }
  return fetchJson<UiMessagesResponse>(`/api/ui/messages?${params.toString()}`);
}

export async function fetchRuntimeProfile() {
  return fetchJson<RuntimeProfile | null>("/api/runtime/profile");
}

export async function saveRuntimeProfile(payload: {
  display_name?: string | null;
  locale?: string | null;
  time_zone?: string | null;
  default_space_id?: string | null;
}) {
  return fetchJson<RuntimeProfile>("/api/runtime/profile", {
    method: "PUT",
    body: JSON.stringify(payload),
  });
}

export async function fetchRuntimePreferences() {
  return fetchJson<UiPreferenceRecord | null>("/api/runtime/preferences");
}

export async function saveInstanceUiPreferences(payload: {
  locale?: string | null;
  theme_id?: string | null;
  time_zone?: string | null;
  date_style?: string | null;
  density?: string | null;
  motion?: string | null;
}) {
  return fetchJson<UiPreferenceRecord>("/api/runtime/preferences", {
    method: "PUT",
    body: JSON.stringify(payload),
  });
}

export async function fetchServerConfig() {
  return fetchJson<ServerConfig>("/api/runtime/server-config");
}

export async function saveServerConfig(payload: ServerConfig) {
  return fetchJson<ServerConfig>("/api/runtime/server-config", {
    method: "PUT",
    body: JSON.stringify(payload),
  });
}

