import { fetchJson } from "./core";
import type { ExtensionScheduleActionContribution } from "@ennoia/ui-sdk";
import type { SchedulePayload, ScheduleRecord } from "./types";

export async function listScheduleActions() {
  return fetchJson<ExtensionScheduleActionContribution[]>("/api/schedule-actions");
}

export async function listSchedules() {
  return fetchJson<ScheduleRecord[]>("/api/schedules");
}

export async function createSchedule(payload: SchedulePayload) {
  return fetchJson<ScheduleRecord>("/api/schedules", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function getSchedule(scheduleId: string) {
  return fetchJson<ScheduleRecord>(`/api/schedules/${scheduleId}`);
}

export async function updateSchedule(scheduleId: string, payload: SchedulePayload) {
  return fetchJson<ScheduleRecord>(`/api/schedules/${scheduleId}`, {
    method: "PUT",
    body: JSON.stringify(payload),
  });
}

export async function deleteSchedule(scheduleId: string) {
  return fetchJson<{ deleted: true }>(`/api/schedules/${scheduleId}`, {
    method: "DELETE",
  });
}

export async function runSchedule(scheduleId: string) {
  return fetchJson<unknown>(`/api/schedules/${scheduleId}/run`, {
    method: "POST",
    body: JSON.stringify({}),
  });
}

export async function pauseSchedule(scheduleId: string) {
  return fetchJson<ScheduleRecord>(`/api/schedules/${scheduleId}/pause`, {
    method: "POST",
    body: JSON.stringify({}),
  });
}

export async function resumeSchedule(scheduleId: string) {
  return fetchJson<ScheduleRecord>(`/api/schedules/${scheduleId}/resume`, {
    method: "POST",
    body: JSON.stringify({}),
  });
}
