import { apiUrl, fetchJson, toQueryString } from "./core";
import type {
  LogStreamDelta,
  LogEntry,
  LogEntryQuery,
  LogsOverview,
  LogTraceDetail,
  LogTraceQuery,
  LogTraceRecord,
  SystemLog,
} from "./types";

export async function listLogs(
  limit = 100,
  filters?: {
    q?: string;
    level?: string;
    source?: string;
  },
) {
  return fetchJson<SystemLog[]>(
    `/api/logs${toQueryString({
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
  return fetchJson<void>("/api/logs/frontend", {
    method: "POST",
  body: JSON.stringify(payload),
  });
}

const LOGS_API = "/api/logs";

export async function getLogsOverview() {
  return fetchJson<LogsOverview>(`${LOGS_API}/overview`);
}

export async function listLogEntries(query: LogEntryQuery = {}) {
  return fetchJson<LogEntry[]>(
    `${LOGS_API}/entries${toQueryString({
      event: query.event,
      level: query.level,
      component: query.component,
      source_kind: query.source_kind,
      source_id: query.source_id,
      request_id: query.request_id,
      trace_id: query.trace_id,
      cursor: query.cursor,
      limit: query.limit,
    })}`,
  );
}

export async function getLogEntryDetail(logId: string) {
  return fetchJson<LogEntry>(`${LOGS_API}/entries/${encodeURIComponent(logId)}`);
}

export async function listLogTraces(query: LogTraceQuery = {}) {
  return fetchJson<LogTraceRecord[]>(
    `${LOGS_API}/traces${toQueryString({
      request_id: query.request_id,
      component: query.component,
      kind: query.kind,
      source_kind: query.source_kind,
      source_id: query.source_id,
      limit: query.limit,
    })}`,
  );
}

export async function getLogTraceDetail(traceId: string) {
  return fetchJson<LogTraceDetail>(`${LOGS_API}/traces/${encodeURIComponent(traceId)}`);
}

export function createLogsStream() {
  return new EventSource(apiUrl(`${LOGS_API}/entries/stream`));
}

export function parseLogsStreamPayload(value: string) {
  return JSON.parse(value) as LogStreamDelta;
}
