import { apiUrl, fetchJson, toQueryString } from "./core";
import type {
  LogStreamDelta,
  ObservationLogEntry,
  ObservationLogQuery,
  ObservationOverview,
  ObservationTraceDetail,
  ObservationTraceQuery,
  ObservationSpanRecord,
} from "./types";

const LOGS_API = "/api/logs";

export async function getObservabilityOverview() {
  return fetchJson<ObservationOverview>(`${LOGS_API}/overview`);
}

export async function listObservabilityLogs(query: ObservationLogQuery = {}) {
  return fetchJson<ObservationLogEntry[]>(
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

export async function getObservabilityLogDetail(logId: string) {
  return fetchJson<ObservationLogEntry>(`${LOGS_API}/entries/${encodeURIComponent(logId)}`);
}

export async function listObservabilityTraces(query: ObservationTraceQuery = {}) {
  return fetchJson<ObservationSpanRecord[]>(
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

export async function getObservabilityTraceDetail(traceId: string) {
  return fetchJson<ObservationTraceDetail>(`${LOGS_API}/traces/${encodeURIComponent(traceId)}`);
}

export function createObservabilityStream() {
  return new EventSource(apiUrl(`${LOGS_API}/entries/stream`));
}

export function parseObservabilityStreamPayload(value: string) {
  return JSON.parse(value) as LogStreamDelta;
}
