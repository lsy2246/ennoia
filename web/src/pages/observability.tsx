import { useEffect, useMemo, useState } from "react";

import {
  getObservabilityLogDetail,
  getObservabilityOverview,
  getObservabilityTraceDetail,
  listObservabilityLogs,
  listObservabilityTraces,
  type ObservationLogEntry,
  type ObservationOverview,
  type ObservationTraceDetail,
  type ObservationSpanRecord,
} from "@ennoia/api-client";
import { Select } from "@/components/Select";
import { useUiHelpers } from "@/stores/ui";

type LogFilters = {
  q: string;
  level: string;
  component: string;
  sourceKind: string;
  requestId: string;
  traceId: string;
};

type TraceFilters = {
  q: string;
  component: string;
  kind: string;
  sourceKind: string;
  requestId: string;
  status: string;
};

type TraceSummary = {
  traceId: string;
  requestId: string;
  name: string;
  component: string;
  kind: string;
  sourceKind: string;
  sourceId?: string | null;
  status: string;
  startedAt: string;
  endedAt: string;
  durationMs: number;
  spanCount: number;
  lastSeq: number;
};

const INITIAL_LOG_FILTERS: LogFilters = {
  q: "",
  level: "",
  component: "",
  sourceKind: "",
  requestId: "",
  traceId: "",
};

const INITIAL_TRACE_FILTERS: TraceFilters = {
  q: "",
  component: "",
  kind: "",
  sourceKind: "",
  requestId: "",
  status: "",
};

function stringifyJson(value: unknown) {
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return String(value);
  }
}

function levelBadgeClass(level: string) {
  switch (level.toLowerCase()) {
    case "error":
    case "fatal":
      return "badge--danger";
    case "warn":
    case "warning":
      return "badge--warn";
    case "info":
    case "debug":
    case "trace":
      return "badge--muted";
    default:
      return "badge--muted";
  }
}

function statusBadgeClass(status: string) {
  const normalized = status.toLowerCase();
  if (normalized.includes("error") || normalized.includes("fail")) {
    return "badge--danger";
  }
  if (normalized.includes("warn") || normalized.includes("timeout") || normalized.includes("cancel")) {
    return "badge--warn";
  }
  if (normalized.includes("ok") || normalized.includes("success") || normalized.includes("done")) {
    return "badge--success";
  }
  return "badge--muted";
}

function collectOptionValues(values: Array<string | null | undefined>) {
  return [...new Set(values.filter((value): value is string => Boolean(value && value.trim())))]
    .sort((left, right) => left.localeCompare(right));
}

function buildTraceSummaries(spans: ObservationSpanRecord[]): TraceSummary[] {
  const grouped = new Map<string, ObservationSpanRecord[]>();
  for (const span of spans) {
    const bucket = grouped.get(span.trace_id);
    if (bucket) {
      bucket.push(span);
    } else {
      grouped.set(span.trace_id, [span]);
    }
  }

  return [...grouped.entries()]
    .map(([traceId, records]) => {
      const ordered = [...records].sort((left, right) => left.seq - right.seq);
      const root = ordered.find((record) => !record.parent_span_id) ?? ordered[0];
      const latest = ordered[ordered.length - 1];
      const startedAt = [...ordered]
        .map((record) => record.started_at)
        .sort((left, right) => left.localeCompare(right))[0] ?? root.started_at;
      const endedAt = [...ordered]
        .map((record) => record.ended_at)
        .sort((left, right) => right.localeCompare(left))[0] ?? latest.ended_at;

      return {
        traceId,
        requestId: root.request_id,
        name: root.name,
        component: root.component,
        kind: root.kind,
        sourceKind: root.source_kind,
        sourceId: root.source_id,
        status: ordered.some((record) => statusBadgeClass(record.status) === "badge--danger")
          ? "error"
          : root.status,
        startedAt,
        endedAt,
        durationMs: root.duration_ms,
        spanCount: ordered.length,
        lastSeq: latest.seq,
      };
    })
    .sort((left, right) => right.lastSeq - left.lastSeq);
}

export function Observability() {
  const { formatDateTime, t } = useUiHelpers();
  const [overview, setOverview] = useState<ObservationOverview | null>(null);
  const [logs, setLogs] = useState<ObservationLogEntry[]>([]);
  const [traceSpans, setTraceSpans] = useState<ObservationSpanRecord[]>([]);
  const [selectedLogId, setSelectedLogId] = useState<string | null>(null);
  const [selectedTraceId, setSelectedTraceId] = useState<string | null>(null);
  const [selectedLog, setSelectedLog] = useState<ObservationLogEntry | null>(null);
  const [selectedTrace, setSelectedTrace] = useState<ObservationTraceDetail | null>(null);
  const [logFilters, setLogFilters] = useState<LogFilters>(INITIAL_LOG_FILTERS);
  const [traceFilters, setTraceFilters] = useState<TraceFilters>(INITIAL_TRACE_FILTERS);
  const [busy, setBusy] = useState(false);
  const [loadingLogDetail, setLoadingLogDetail] = useState(false);
  const [loadingTraceDetail, setLoadingTraceDetail] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const traceSummaries = useMemo(() => buildTraceSummaries(traceSpans), [traceSpans]);

  const logLevelOptions = useMemo(() => collectOptionValues(logs.map((item) => item.level)), [logs]);
  const logComponentOptions = useMemo(() => collectOptionValues(logs.map((item) => item.component)), [logs]);
  const logSourceKindOptions = useMemo(() => collectOptionValues(logs.map((item) => item.source_kind)), [logs]);

  const traceComponentOptions = useMemo(() => collectOptionValues(traceSummaries.map((item) => item.component)), [traceSummaries]);
  const traceKindOptions = useMemo(() => collectOptionValues(traceSummaries.map((item) => item.kind)), [traceSummaries]);
  const traceSourceKindOptions = useMemo(() => collectOptionValues(traceSummaries.map((item) => item.sourceKind)), [traceSummaries]);
  const traceStatusOptions = useMemo(() => collectOptionValues(traceSummaries.map((item) => item.status)), [traceSummaries]);

  const filteredLogs = useMemo(() => {
    return logs.filter((item) => {
      if (logFilters.level && item.level !== logFilters.level) {
        return false;
      }
      if (logFilters.component && item.component !== logFilters.component) {
        return false;
      }
      if (logFilters.sourceKind && item.source_kind !== logFilters.sourceKind) {
        return false;
      }
      if (logFilters.requestId && item.request_id !== logFilters.requestId.trim()) {
        return false;
      }
      if (logFilters.traceId && item.trace_id !== logFilters.traceId.trim()) {
        return false;
      }
      if (!logFilters.q.trim()) {
        return true;
      }
      const haystack = [
        item.event,
        item.message,
        item.component,
        item.source_kind,
        item.source_id,
        item.request_id,
        item.trace_id,
        stringifyJson(item.attributes),
      ]
        .filter(Boolean)
        .join("\n")
        .toLowerCase();
      return haystack.includes(logFilters.q.trim().toLowerCase());
    });
  }, [logFilters, logs]);

  const filteredTraces = useMemo(() => {
    return traceSummaries.filter((item) => {
      if (traceFilters.component && item.component !== traceFilters.component) {
        return false;
      }
      if (traceFilters.kind && item.kind !== traceFilters.kind) {
        return false;
      }
      if (traceFilters.sourceKind && item.sourceKind !== traceFilters.sourceKind) {
        return false;
      }
      if (traceFilters.requestId && item.requestId !== traceFilters.requestId.trim()) {
        return false;
      }
      if (traceFilters.status && item.status !== traceFilters.status) {
        return false;
      }
      if (!traceFilters.q.trim()) {
        return true;
      }
      const haystack = [
        item.traceId,
        item.requestId,
        item.name,
        item.component,
        item.kind,
        item.sourceKind,
        item.sourceId,
      ]
        .filter(Boolean)
        .join("\n")
        .toLowerCase();
      return haystack.includes(traceFilters.q.trim().toLowerCase());
    });
  }, [traceFilters, traceSummaries]);

  async function refresh() {
    setBusy(true);
    setError(null);
    try {
      const [nextOverview, nextLogs, nextTraces] = await Promise.all([
        getObservabilityOverview(),
        listObservabilityLogs({ limit: 160 }),
        listObservabilityTraces({ limit: 200 }),
      ]);
      const nextTraceSummaries = buildTraceSummaries(nextTraces);
      setOverview(nextOverview);
      setLogs(nextLogs);
      setTraceSpans(nextTraces);
      setSelectedLogId((current) => nextLogs.some((item) => item.id === current) ? current : nextLogs[0]?.id ?? null);
      setSelectedTraceId((current) =>
        nextTraceSummaries.some((item) => item.traceId === current) ? current : nextTraceSummaries[0]?.traceId ?? null);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  useEffect(() => {
    void refresh();
  }, []);

  useEffect(() => {
    setSelectedLogId((current) =>
      filteredLogs.some((item) => item.id === current) ? current : filteredLogs[0]?.id ?? null);
  }, [filteredLogs]);

  useEffect(() => {
    setSelectedTraceId((current) =>
      filteredTraces.some((item) => item.traceId === current) ? current : filteredTraces[0]?.traceId ?? null);
  }, [filteredTraces]);

  useEffect(() => {
    if (!selectedLogId) {
      setSelectedLog(null);
      return;
    }

    let cancelled = false;
    setLoadingLogDetail(true);
    void getObservabilityLogDetail(selectedLogId)
      .then((detail) => {
        if (!cancelled) {
          setSelectedLog(detail);
        }
      })
      .catch((err) => {
        if (!cancelled) {
          setError(String(err));
        }
      })
      .finally(() => {
        if (!cancelled) {
          setLoadingLogDetail(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [selectedLogId]);

  useEffect(() => {
    if (!selectedTraceId) {
      setSelectedTrace(null);
      return;
    }

    let cancelled = false;
    setLoadingTraceDetail(true);
    void getObservabilityTraceDetail(selectedTraceId)
      .then((detail) => {
        if (!cancelled) {
          setSelectedTrace(detail);
        }
      })
      .catch((err) => {
        if (!cancelled) {
          setError(String(err));
        }
      })
      .finally(() => {
        if (!cancelled) {
          setLoadingTraceDetail(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [selectedTraceId]);

  return (
    <div className="observability-layout">
      <section className="work-panel">
        <div className="page-heading">
          <span>{t("web.observability.eyebrow", "Observability")}</span>
          <h1>{t("web.observability.title", "统一查看宿主日志、trace 链路和系统运行脉络。")}</h1>
          <p>{t("web.observability.description", "页面展示最近样本；筛选作用于当前已加载的数据，点击刷新会重新拉取最新记录。")}</p>
        </div>
        {error ? <div className="error">{error}</div> : null}
        <div className="metric-grid">
          <article className="metric-card">
            <span>{t("web.observability.metrics.logs", "日志总量")}</span>
            <strong>{overview?.log_count ?? "—"}</strong>
          </article>
          <article className="metric-card">
            <span>{t("web.observability.metrics.spans", "Span 总量")}</span>
            <strong>{overview?.span_count ?? "—"}</strong>
          </article>
          <article className="metric-card">
            <span>{t("web.observability.metrics.traces", "Trace 总量")}</span>
            <strong>{overview?.trace_count ?? "—"}</strong>
          </article>
          <article className="metric-card">
            <span>{t("web.observability.metrics.loaded", "当前加载")}</span>
            <strong>{logs.length + traceSummaries.length}</strong>
          </article>
        </div>
      </section>

      <div className="observability-grid">
        <section className="work-panel observability-panel">
          <div className="observability-section-header">
            <div className="page-heading">
              <span>{t("web.observability.logs.eyebrow", "Logs")}</span>
              <h1>{t("web.observability.logs.title", "最近日志")}</h1>
              <p>{t("web.observability.logs.description", "按等级、组件、来源或 request/trace 精确定位，再展开看属性详情。")}</p>
            </div>
            <button type="button" className="secondary" onClick={() => void refresh()} disabled={busy}>
              {busy ? t("web.common.loading", "加载中…") : t("web.action.refresh", "刷新")}
            </button>
          </div>

          <div className="observability-filter-grid">
            <input
              value={logFilters.q}
              onChange={(event) => setLogFilters((current) => ({ ...current, q: event.target.value }))}
              placeholder={t("web.observability.logs.search", "搜索 event、message、trace_id 或 attributes")}
            />
            <Select
              value={logFilters.level}
              onChange={(value) => setLogFilters((current) => ({ ...current, level: value }))}
              placeholder={t("web.observability.logs.level", "全部等级")}
              options={[
                { value: "", label: t("web.observability.logs.level", "全部等级") },
                ...logLevelOptions.map((item) => ({ value: item, label: item })),
              ]}
            />
            <Select
              value={logFilters.component}
              onChange={(value) => setLogFilters((current) => ({ ...current, component: value }))}
              placeholder={t("web.observability.logs.component", "全部组件")}
              options={[
                { value: "", label: t("web.observability.logs.component", "全部组件") },
                ...logComponentOptions.map((item) => ({ value: item, label: item })),
              ]}
            />
            <Select
              value={logFilters.sourceKind}
              onChange={(value) => setLogFilters((current) => ({ ...current, sourceKind: value }))}
              placeholder={t("web.observability.logs.source_kind", "全部来源类型")}
              options={[
                { value: "", label: t("web.observability.logs.source_kind", "全部来源类型") },
                ...logSourceKindOptions.map((item) => ({ value: item, label: item })),
              ]}
            />
            <input
              value={logFilters.requestId}
              onChange={(event) => setLogFilters((current) => ({ ...current, requestId: event.target.value }))}
              placeholder={t("web.observability.logs.request_id", "按 request_id 过滤")}
            />
            <input
              value={logFilters.traceId}
              onChange={(event) => setLogFilters((current) => ({ ...current, traceId: event.target.value }))}
              placeholder={t("web.observability.logs.trace_id", "按 trace_id 过滤")}
            />
          </div>

          <div className="observability-scroll stack">
            {filteredLogs.length === 0 ? (
              <div className="empty-card">{t("web.observability.logs.empty", "当前筛选下没有日志。")}</div>
            ) : (
              filteredLogs.map((item) => (
                <article
                  key={item.id}
                  className={`log-card observability-item ${selectedLogId === item.id ? "observability-item--active" : ""}`}
                  onClick={() => setSelectedLogId(item.id)}
                  role="button"
                  tabIndex={0}
                  onKeyDown={(event) => {
                    if (event.key === "Enter" || event.key === " ") {
                      event.preventDefault();
                      setSelectedLogId(item.id);
                    }
                  }}
                >
                  <header>
                    <strong>{item.event}</strong>
                    <span className="observability-meta">
                      <span className={`badge ${levelBadgeClass(item.level)}`}>{item.level}</span>
                      <span className="badge badge--muted">{item.component}</span>
                      <span className="badge badge--muted">{item.source_kind}</span>
                    </span>
                  </header>
                  <p>{item.message}</p>
                  <small>{formatDateTime(item.created_at)}</small>
                </article>
              ))
            )}
          </div>

          <div className="details-panel observability-detail">
            <div className="observability-section-header">
              <strong>{t("web.observability.logs.detail", "日志详情")}</strong>
              {loadingLogDetail ? <span>{t("web.common.loading", "加载中…")}</span> : null}
            </div>
            {selectedLog ? (
              <div className="stack">
                <div className="kv-list">
                  <span>ID</span><strong>{selectedLog.id}</strong>
                  <span>event</span><strong>{selectedLog.event}</strong>
                  <span>level</span><strong><span className={`badge ${levelBadgeClass(selectedLog.level)}`}>{selectedLog.level}</span></strong>
                  <span>component</span><strong>{selectedLog.component}</strong>
                  <span>source</span><strong>{selectedLog.source_kind}{selectedLog.source_id ? `:${selectedLog.source_id}` : ""}</strong>
                  <span>request_id</span><strong>{selectedLog.request_id ?? "—"}</strong>
                  <span>trace_id</span><strong>{selectedLog.trace_id ?? "—"}</strong>
                  <span>span_id</span><strong>{selectedLog.span_id ?? "—"}</strong>
                </div>
                <div className="stack">
                  <div>
                    <strong>{t("web.observability.common.message", "消息")}</strong>
                    <p>{selectedLog.message}</p>
                  </div>
                  <div>
                    <strong>{t("web.observability.common.attributes", "属性")}</strong>
                    <pre className="observability-json">{stringifyJson(selectedLog.attributes)}</pre>
                  </div>
                </div>
              </div>
            ) : (
              <div className="empty-card">{t("web.observability.logs.detail_empty", "选择一条日志后在这里查看完整详情。")}</div>
            )}
          </div>
        </section>

        <section className="work-panel observability-panel">
          <div className="observability-section-header">
            <div className="page-heading">
              <span>{t("web.observability.traces.eyebrow", "Traces")}</span>
              <h1>{t("web.observability.traces.title", "最近 traces")}</h1>
              <p>{t("web.observability.traces.description", "以 trace 为单位汇总 span，再展开查看链路顺序和 link 关系。")}</p>
            </div>
            <button type="button" className="secondary" onClick={() => void refresh()} disabled={busy}>
              {busy ? t("web.common.loading", "加载中…") : t("web.action.refresh", "刷新")}
            </button>
          </div>

          <div className="observability-filter-grid">
            <input
              value={traceFilters.q}
              onChange={(event) => setTraceFilters((current) => ({ ...current, q: event.target.value }))}
              placeholder={t("web.observability.traces.search", "搜索 trace_id、name、component 或 request_id")}
            />
            <Select
              value={traceFilters.component}
              onChange={(value) => setTraceFilters((current) => ({ ...current, component: value }))}
              placeholder={t("web.observability.traces.component", "全部组件")}
              options={[
                { value: "", label: t("web.observability.traces.component", "全部组件") },
                ...traceComponentOptions.map((item) => ({ value: item, label: item })),
              ]}
            />
            <Select
              value={traceFilters.kind}
              onChange={(value) => setTraceFilters((current) => ({ ...current, kind: value }))}
              placeholder={t("web.observability.traces.kind", "全部 kind")}
              options={[
                { value: "", label: t("web.observability.traces.kind", "全部 kind") },
                ...traceKindOptions.map((item) => ({ value: item, label: item })),
              ]}
            />
            <Select
              value={traceFilters.sourceKind}
              onChange={(value) => setTraceFilters((current) => ({ ...current, sourceKind: value }))}
              placeholder={t("web.observability.traces.source_kind", "全部来源类型")}
              options={[
                { value: "", label: t("web.observability.traces.source_kind", "全部来源类型") },
                ...traceSourceKindOptions.map((item) => ({ value: item, label: item })),
              ]}
            />
            <input
              value={traceFilters.requestId}
              onChange={(event) => setTraceFilters((current) => ({ ...current, requestId: event.target.value }))}
              placeholder={t("web.observability.traces.request_id", "按 request_id 过滤")}
            />
            <Select
              value={traceFilters.status}
              onChange={(value) => setTraceFilters((current) => ({ ...current, status: value }))}
              placeholder={t("web.observability.traces.status", "全部状态")}
              options={[
                { value: "", label: t("web.observability.traces.status", "全部状态") },
                ...traceStatusOptions.map((item) => ({ value: item, label: item })),
              ]}
            />
          </div>

          <div className="observability-scroll stack">
            {filteredTraces.length === 0 ? (
              <div className="empty-card">{t("web.observability.traces.empty", "当前筛选下没有 trace。")}</div>
            ) : (
              filteredTraces.map((item) => (
                <article
                  key={item.traceId}
                  className={`resource-card observability-item ${selectedTraceId === item.traceId ? "observability-item--active" : ""}`}
                  onClick={() => setSelectedTraceId(item.traceId)}
                  role="button"
                  tabIndex={0}
                  onKeyDown={(event) => {
                    if (event.key === "Enter" || event.key === " ") {
                      event.preventDefault();
                      setSelectedTraceId(item.traceId);
                    }
                  }}
                >
                  <header>
                    <strong>{item.name}</strong>
                    <span className="observability-meta">
                      <span className={`badge ${statusBadgeClass(item.status)}`}>{item.status}</span>
                      <span className="badge badge--muted">{item.component}</span>
                      <span className="badge badge--muted">{item.kind}</span>
                    </span>
                  </header>
                  <p>{item.traceId}</p>
                  <small>
                    {item.spanCount} spans · {item.durationMs} ms · {formatDateTime(item.endedAt)}
                  </small>
                </article>
              ))
            )}
          </div>

          <div className="details-panel observability-detail">
            <div className="observability-section-header">
              <strong>{t("web.observability.traces.detail", "Trace 详情")}</strong>
              {loadingTraceDetail ? <span>{t("web.common.loading", "加载中…")}</span> : null}
            </div>
            {selectedTrace ? (
              <div className="stack">
                <div className="kv-list">
                  <span>trace_id</span><strong>{selectedTrace.trace_id}</strong>
                  <span>spans</span><strong>{selectedTrace.spans.length}</strong>
                  <span>links</span><strong>{selectedTrace.links.length}</strong>
                  <span>started_at</span><strong>{formatDateTime(selectedTrace.spans[0]?.started_at ?? "")}</strong>
                </div>

                {selectedTrace.links.length > 0 ? (
                  <div className="stack">
                    <strong>{t("web.observability.traces.links", "Trace links")}</strong>
                    {selectedTrace.links.map((link) => (
                      <article key={link.id} className="mini-card">
                        <strong>{link.link_type}</strong>
                        <span>{link.span_id} → {link.linked_span_id}</span>
                        <span>{link.linked_trace_id}</span>
                      </article>
                    ))}
                  </div>
                ) : null}

                <div className="timeline-list">
                  {selectedTrace.spans.map((span) => (
                    <article key={span.id} className="timeline-item">
                      <div className="stack">
                        <strong>{span.duration_ms} ms</strong>
                        <small>{formatDateTime(span.started_at)}</small>
                      </div>
                      <div className="stack">
                        <div className="observability-section-header">
                          <strong>{span.name}</strong>
                          <span className="observability-meta">
                            <span className={`badge ${statusBadgeClass(span.status)}`}>{span.status}</span>
                            <span className="badge badge--muted">{span.kind}</span>
                          </span>
                        </div>
                        <div className="observability-meta">
                          <span className="badge badge--muted">{span.component}</span>
                          <span className="badge badge--muted">{span.source_kind}</span>
                          {span.source_id ? <span className="badge badge--muted">{span.source_id}</span> : null}
                          {span.parent_span_id ? <span className="badge badge--muted">parent:{span.parent_span_id}</span> : null}
                        </div>
                        <pre className="observability-json">{stringifyJson(span.attributes)}</pre>
                      </div>
                    </article>
                  ))}
                </div>
              </div>
            ) : (
              <div className="empty-card">{t("web.observability.traces.detail_empty", "选择一条 trace 后在这里查看 span 链路。")}</div>
            )}
          </div>
        </section>
      </div>
    </div>
  );
}
