import { fetchJson, toQueryString } from "./core";
import type { SystemLog } from "./types";

export async function listLogs(
  limit = 100,
  filters?: {
    q?: string;
    level?: string;
    source?: string;
  },
) {
  return fetchJson<SystemLog[]>(
    `/api/v1/logs${toQueryString({
      limit,
      q: filters?.q,
      level: filters?.level,
      source: filters?.source,
    })}`,
  );
}

export async function reportFrontendLog(payload: {
  level: string;
  title: string;
  summary: string;
  source?: string;
  details?: string;
  at?: string;
}) {
  return fetchJson<void>("/api/v1/logs/frontend", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

