import { fetchJson } from "./core";
import type { ExecutionRun, TaskJob, TaskJobDetail } from "./types";

export async function listTaskJobs() {
  return fetchJson<TaskJob[]>("/api/v1/jobs");
}

export async function listRuns() {
  return fetchJson<ExecutionRun[]>("/api/v1/runs");
}

export async function getTaskJob(jobId: string) {
  return fetchJson<TaskJobDetail>(`/api/v1/jobs/${jobId}`);
}

export async function createTaskJob(payload: {
  owner_kind: string;
  owner_id: string;
  job_kind?: string;
  schedule_kind: string;
  schedule_value: string;
  payload?: unknown;
  max_retries?: number;
  run_at?: string;
}) {
  return fetchJson<TaskJobDetail>("/api/v1/jobs", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function updateTaskJob(
  jobId: string,
  payload: {
    job_kind?: string;
    schedule_kind?: string;
    schedule_value?: string;
    payload?: unknown;
    max_retries?: number;
    run_at?: string;
  },
) {
  return fetchJson<TaskJobDetail>(`/api/v1/jobs/${jobId}`, {
    method: "PUT",
    body: JSON.stringify(payload),
  });
}

export async function deleteTaskJob(jobId: string) {
  return fetchJson<void>(`/api/v1/jobs/${jobId}`, { method: "DELETE" });
}

export async function runTaskJobNow(jobId: string) {
  return fetchJson<TaskJobDetail>(`/api/v1/jobs/${jobId}/run`, {
    method: "POST",
    body: JSON.stringify({}),
  });
}

export async function enableTaskJob(jobId: string) {
  return fetchJson<TaskJobDetail>(`/api/v1/jobs/${jobId}/enable`, {
    method: "POST",
    body: JSON.stringify({}),
  });
}

export async function disableTaskJob(jobId: string) {
  return fetchJson<TaskJobDetail>(`/api/v1/jobs/${jobId}/disable`, {
    method: "POST",
    body: JSON.stringify({}),
  });
}

