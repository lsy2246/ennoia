import { fetchJson } from "./core";
import type {
  ExtensionActionContribution,
  ExtensionScheduleActionContribution,
} from "@ennoia/ui-sdk";
import type { ActionStatus } from "./types";

export async function listExtensionActions() {
  return fetchJson<ExtensionActionContribution[]>("/api/extensions/actions");
}

export async function listExtensionScheduleActions() {
  return fetchJson<ExtensionScheduleActionContribution[]>("/api/extensions/schedule-actions");
}

export async function listActionStatus() {
  return fetchJson<ActionStatus[]>("/api/actions");
}
