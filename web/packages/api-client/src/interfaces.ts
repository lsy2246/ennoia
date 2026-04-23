import { fetchJson } from "./core";
import type {
  ExtensionInterfaceContribution,
  ExtensionScheduleActionContribution,
} from "@ennoia/ui-sdk";
import type { InterfaceBindings, InterfaceStatus } from "./types";

export async function listExtensionInterfaces() {
  return fetchJson<ExtensionInterfaceContribution[]>("/api/extensions/interfaces");
}

export async function listExtensionScheduleActions() {
  return fetchJson<ExtensionScheduleActionContribution[]>("/api/extensions/schedule-actions");
}

export async function listInterfaceStatus() {
  return fetchJson<InterfaceStatus[]>("/api/interfaces");
}

export async function fetchInterfaceBindings() {
  return fetchJson<InterfaceBindings>("/api/interfaces/bindings");
}

export async function saveInterfaceBindings(payload: InterfaceBindings) {
  return fetchJson<InterfaceBindings>("/api/interfaces/bindings", {
    method: "PUT",
    body: JSON.stringify(payload),
  });
}
