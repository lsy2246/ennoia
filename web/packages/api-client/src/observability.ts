import { fetchJson, toQueryString } from "./core";
import type {
  ObservationLogEntry,
  ObservationLogQuery,
  ObservationOverview,
  ObservationTraceDetail,
  ObservationTraceQuery,
  ObservationSpanRecord,
} from "./types";

const OBSERVABILITY_API = "/api/observability";

export async function getObservabilityOverview() {
  return fetchJson<ObservationOverview>(`${OBSERVABILITY_API}/overview`);
}

export async function listObservabilityLogs(query: ObservationLogQuery = {}) {
  return fetchJson<ObservationLogEntry[]>(
    `${OBSERVABILITY_API}/logs${toQueryString({
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
  return fetchJson<ObservationLogEntry>(`${OBSERVABILITY_API}/logs/${encodeURIComponent(logId)}`);
}

export async function listObservabilityTraces(query: ObservationTraceQuery = {}) {
  return fetchJson<ObservationSpanRecord[]>(
    `${OBSERVABILITY_API}/traces${toQueryString({
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
  return fetchJson<ObservationTraceDetail>(`${OBSERVABILITY_API}/traces/${encodeURIComponent(traceId)}`);
}
